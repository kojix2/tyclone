require "json"
require "./spec_helper"

describe "PhyClone compat kernel" do
  it "raises a line-numbered CliError for invalid phy input numbers" do
    path = File.join(Dir.tempdir, "tyclone_phy_input_#{Random.rand(1_000_000)}.tsv")
    begin
      File.write(
        path,
        "mutation_id\tsample_id\tref_counts\talt_counts\tmajor_cn\tminor_cn\tnormal_cn\n" +
        "m1\ts1\t10\t5\t2\t1\t2\n" +
        "m2\ts1\t12\t4\t2\t1\tbad\n"
      )

      expect_raises(Tyclone::CliError, /Line 3: invalid integer for 'normal_cn': bad/) do
        Tyclone::PhyClone::Input.read_tsv(path)
      end
    ensure
      File.delete(path) if File.exists?(path)
    end
  end

  it "generates JSONL trace records through the Rust compat path" do
    rows = [
      Tyclone::PhyClone::InputRow.new("m1", "s1", 10, 5, 2, 1, 2, 1.0, 0.001, nil, "1", nil, nil),
      Tyclone::PhyClone::InputRow.new("m2", "s1", 12, 4, 2, 1, 2, 1.0, 0.001, nil, "1", nil, nil),
    ]
    config = Tyclone::PhyCloneRunConfig.new
    config.num_particles = 2
    config.burn_in_iters = 0
    config.num_grid_points = 11
    config.print_freq = 1

    jsonl = Tyclone::PhyCloneKernel.generate_trace(rows, nil, config, 1, 2, 7_u64)
    records = jsonl.lines.map { |line| JSON.parse(line) }

    records.size.should eq(2)
    records.each do |record|
      record["schema_version"].as_i.should eq(Tyclone::PhyClone::TRACE_SCHEMA_VERSION)
      record["clusters"].as_a.size.should eq(2)
    end
  end
end
