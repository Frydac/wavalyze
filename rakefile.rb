namespace :channels do
    desc 'Validate and import workplace channel JSON, then regenerate Rust (requires source_dir)'
    task :import, [:source_dir] do |_, args|
        abort 'Usage: rake "channels:import[source_dir]"' unless args[:source_dir] && !args[:source_dir].empty?

        ruby File.expand_path('scripts/generate_channels.rb', __dir__), '--import', args[:source_dir]
    end

    desc 'Generate Rust channel IDs and layouts from the local JSON definitions'
    task :generate do
        ruby File.expand_path('scripts/generate_channels.rb', __dir__)
    end

    desc 'Check that generated Rust channel IDs and layouts match the local JSON definitions'
    task :check do
        ruby File.expand_path('scripts/generate_channels.rb', __dir__), '--check'
    end
end

namespace :test_data do
    desc 'Generate basic audio fixtures with SoX'
    task :basic do
        ruby File.expand_path('scripts/generate_test_data.rb', __dir__), '--kind', 'basic'
    end

    desc 'Generate PDF-convention WAVEX fixtures with explicit channel masks'
    task :wavex do
        ruby File.expand_path('scripts/generate_test_data.rb', __dir__), '--kind', 'wavex'
    end
end

desc 'Generate basic and WAVEX audio fixtures'
task create_test_data: ['test_data:basic', 'test_data:wavex']
