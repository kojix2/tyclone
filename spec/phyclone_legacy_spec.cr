require "./spec_helper"

describe "PhyClone legacy spec coverage" do
  it "keeps legacy coverage consolidated in phyclone_spec" do
    File.exists?(File.join(__DIR__, "phyclone_spec.cr")).should be_true
  end
end
