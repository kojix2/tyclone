module Toyclone
  enum Density
    Binomial
    BetaBinomial
  end

  enum Action
    Fit
    Help
    Version
  end

  class Config
    property action : Action
    property command : String
    property in_file : String
    property out_file : String
    property num_clusters : Int32
    property density : Density
    property num_grid_points : Int32
    property num_restarts : Int32
    property convergence_threshold : Float64
    property max_iters : Int32
    property mix_weight_prior : Float64
    property precision : Float64
    property print_freq : Int32
    property seed : UInt64?
    property kernel_threads : Int32
    property restart_parallelism : Int32
    property? compress : Bool
    property help_message : String

    def initialize
      @action = Action::Fit
      @command = "fit"
      @in_file = ""
      @out_file = ""
      @num_clusters = 10
      @density = Density::Binomial
      @num_grid_points = 100
      @num_restarts = 1
      @convergence_threshold = 1e-6
      @max_iters = 10_000
      @mix_weight_prior = 1.0
      @precision = 200.0
      @print_freq = 100
      @seed = nil
      @kernel_threads = 1
      @restart_parallelism = 1
      @compress = false
      @help_message = ""
    end
  end
end
