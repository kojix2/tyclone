require "./spec_helper"

describe Toyclone do
  it "has a version" do
    Toyclone::VERSION.should_not be_empty
  end

  it "parses fit command" do
    config = Toyclone::CLI.parse(["fit", "-i", "in.tsv", "-o", "out.tsv"])
    config.command.should eq("fit")
    config.action.should eq(Toyclone::Action::Fit)
    config.in_file.should eq("in.tsv")
    config.out_file.should eq("out.tsv")
  end

  it "parses --help" do
    config = Toyclone::CLI.parse(["--help"])
    config.action.should eq(Toyclone::Action::Help)
    config.help_message.should contain("Usage: toyclone")
  end

  it "parses --version" do
    config = Toyclone::CLI.parse(["--version"])
    config.action.should eq(Toyclone::Action::Version)
  end

  it "raises on missing command" do
    expect_raises(Toyclone::CliError, /Missing command/) do
      Toyclone::CLI.parse([] of String)
    end
  end
end
