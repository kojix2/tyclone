module Toyclone
  module Run
    def self.execute(config : Config)
      rows = Input.read_tsv(config.in_file)
      sanitized_rows = Sanitize.run(rows)
      if sanitized_rows.empty?
        raise CliError.new("No valid rows remain after sanitization")
      end

      indexed = Indexing.build(sanitized_rows)
      result = case config.engine
               when Engine::MCMC
                 Kernel.fit_mcmc(config, indexed.rows, indexed.num_mutations, indexed.num_samples)
               else
                 Kernel.fit(config, indexed.rows, indexed.num_mutations, indexed.num_samples)
               end

      begin
        out_rows = ResultBuilder.build(indexed, result)
        Output.write(config.out_file, out_rows, config.compress?)
      ensure
        result.free
      end
    end
  end
end
