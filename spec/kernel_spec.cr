require "./spec_helper"

private def default_config
  config = UnClone::ViConfig.new
  config.in_file = "in.tsv"
  config.out_file = "out.tsv"
  config.num_clusters = 10
  config.density = UnClone::Density::BetaBinomial
  config.num_grid_points = 100
  config.num_restarts = 1
  config.convergence_threshold = 1e-6
  config.max_iters = 100
  config.mix_weight_prior = 1.0
  config.precision = 1000.0
  config.print_freq = 10
  config.seed = 1_u64
  config.kernel_threads = 1
  config.restart_parallelism = 1
  config.compress = false
  config
end

describe UnClone::Kernel do
  it "propagates kernel errors" do
    expect_raises(UnClone::KernelError, /config or rows is null/) do
      UnClone::Kernel.fit(default_config, [] of UnClone::IndexedRow, 0, 0)
    end
  end
end
