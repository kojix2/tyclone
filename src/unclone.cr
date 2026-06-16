require "set"
require "./unclone/config"
require "./unclone/errors"
require "./unclone/input"
require "./unclone/sanitize"
require "./unclone/indexing"
require "./unclone/ffi"
require "./unclone/kernel_result"
require "./unclone/kernel"
require "./unclone/phyclone_kernel"
require "./unclone/phyclone"
require "./unclone/vi_kernel"
require "./unclone/result"
require "./unclone/output"
require "./unclone/cli"
require "./unclone/run"

module UnClone
  VERSION      = {{ `shards version #{__DIR__}`.chomp.stringify }}
  DISPLAY_NAME = "UnClone"
  PROGRAM      = "unclone"
  SOURCE       = "https://github.com/kojix2/unclone"

  def self.main(args = ARGV)
    parser = CLI::Parser.new
    command = parser.parse(args)
    dispatch_command(command)
  rescue ex : CliError | OptionParser::Exception
    STDERR.puts("error: #{ex.message}")
    parser = CLI::Parser.new
    STDERR.puts(parser.to_s)
    exit 1
  rescue ex : KernelError
    STDERR.puts("error: #{ex.message}")
    exit 1
  end

  private def self.dispatch_command(command)
    case command
    when HelpCommand
      puts command.help_message
    when VersionCommand
      puts "#{PROGRAM} #{VERSION}"
    when FitViCommand,
         PhyCloneRunCommand,
         PhyCloneMapCommand,
         PhyCloneConsensusCommand,
         PhyCloneTopologyReportCommand
      Run.execute(command.config)
    end
  end
end
