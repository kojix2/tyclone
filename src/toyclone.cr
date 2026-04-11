require "set"
require "./pyclone_vi/config"
require "./pyclone_vi/errors"
require "./pyclone_vi/input"
require "./pyclone_vi/sanitize"
require "./pyclone_vi/kernel"
require "./pyclone_vi/indexing"
require "./pyclone_vi/result"
require "./pyclone_vi/output"
require "./pyclone_vi/cli"
require "./pyclone_vi/run"

module Toyclone
  VERSION = {{ `shards version #{__DIR__}`.chomp.stringify }}
  PROGRAM = "toyclone"

  def self.main(args = ARGV)
    parser = CLI::Parser.new
    config = parser.parse(args)

    case config.action
    when Action::Help
      puts config.help_message
    when Action::Version
      puts "#{PROGRAM} #{VERSION}"
    when Action::Fit
      Run.execute(config)
    end
  rescue ex : CliError | OptionParser::Exception
    STDERR.puts("error: #{ex.message}")
    parser = CLI::Parser.new
    STDERR.puts(parser.help_message)
    exit 1
  rescue ex : KernelError
    STDERR.puts("error: #{ex.message}")
    exit 1
  end
end
