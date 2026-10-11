# frozen_string_literal: true

require 'fileutils'
require 'json'
require 'optparse'

module TestData
  DEFAULT_OUTPUT_DIR = File.expand_path('../data', __dir__)

  def self.generate_basic(output_dir: DEFAULT_OUTPUT_DIR)
    [
      {
        bit_depth: 16,
        sample_type: SampleType::SIGNED,
        sample_rate: 48_000,
        nr_channels: 1,
      },
      {
        bit_depth: 24,
        sample_type: SampleType::SIGNED,
        sample_rate: 48_000,
        nr_channels: 1,
      },
      {
        bit_depth: 32,
        sample_type: SampleType::FLOAT,
        sample_rate: 48_000,
        nr_channels: 1,
      },
      {
        bit_depth: 16,
        sample_type: SampleType::SIGNED,
        sample_rate: 48_000,
        nr_channels: 2,
      },
      {
        bit_depth: 24,
        sample_type: SampleType::SIGNED,
        sample_rate: 48_000,
        nr_channels: 2,
      },
      {
        bit_depth: 32,
        sample_type: SampleType::SIGNED,
        sample_rate: 48_000,
        nr_channels: 2,
      },
      {
        bit_depth: 16,
        sample_type: SampleType::SIGNED,
        sample_rate: 44_100,
        nr_channels: 10,
      },
      {
        bit_depth: 32,
        sample_type: SampleType::FLOAT,
        sample_rate: 48_000,
        nr_channels: 3,
      },
      {
        bit_depth: 32,
        sample_type: SampleType::FLOAT,
        sample_rate: 48_000,
        nr_channels: 5,
      },
      {
        bit_depth: 16,
        sample_type: SampleType::SIGNED,
        signal: SignalType::SQUARE,
        sample_rate: 48_000,
        nr_channels: 2,
      },
    ].each do |kwargs|
      default_signal = { signal: SignalType::SINE }
      kwargs = default_signal.merge(kwargs)
      fn = data_dir(output_dir, file: "#{kwargs[:signal]}_#{kwargs[:bit_depth]}_#{kwargs[:sample_type]}_#{kwargs[:sample_rate]}_#{kwargs[:nr_channels]}.wav")
      SoundFileBuilder.new.set(**kwargs).build(fn)
    end
  end

  # Literal orders from Auro-3D in multi-channel WAV, Revision 2 (2024-04-19).
  # 22.0 is derived from 22.2 by omitting LFE (bit 3) and LFE2 (bit 18).
  # WAVEX masks are independent of imported internal layout masks and ID numbers.
  WAVEX_LAYOUTS = [
    ['stereo_2_0', '2.0', 0x000003, %w[L R]],
    ['quad_4_0', '4.0', 0x000603, %w[L R Ls Rs]],
    ['surround_5_1', '5.1', 0x00060f, %w[L R C LFE Ls Rs]],
    ['auro_9_1', 'Auro9.1', 0x02d60f, %w[L R C LFE Ls Rs HL HR HLs HRs]],
    ['auro_13_1', 'Auro13.1', 0x02fe3f, %w[L R C LFE Lb Rb Ls Rs T HL HC HR HLs HRs]],
    ['22_0', 'NHK_22.0', 0xfbfff7,
     %w[L R C Lb Rb LC RC Cs Ls Rs T HL HC HR HLb HCs HRb HLs HRs BL BC BR]],
    ['22_2', 'NHK_22.2', 0xffffff,
     %w[L R C LFE Lb Rb LC RC Cs Ls Rs T HL HC HR HLb HCs HRb LFE2 HLs HRs BL BC BR]]
  ].freeze

  def self.generate_wavex(output_dir: DEFAULT_OUTPUT_DIR, duration: 1.0)
    dir = data_dir(File.join(output_dir, 'wavex'))
    sample_rate = 48_000
    frames = (sample_rate * duration).round
    raise ArgumentError, 'Duration must produce at least one sample frame' unless frames.positive?

    entries = WAVEX_LAYOUTS.map do |stem, layout_name, mask, ids|
      # Ascending WAVEX-bit order; distinct tones make channel permutations visible.
      channels = ids.each_with_index.map do |id, index|
        { index: index, id: id, frequency_hz: 440 + index * 200 }
      end
      filename = "wavex_#{stem}.wav"
      write_wavex(File.join(dir, filename), mask, channels, sample_rate, frames)
      puts "Generated #{filename}: #{layout_name}, mask 0x#{format('%06X', mask)}, #{ids.length} channels"
      {
        file: filename, layout: layout_name, channel_mask: format('0x%06X', mask),
        channels: channels
      }
    end
    manifest = {
      convention: 'Auro-3D in multi-channel WAV, Revision 2 (2024-04-19); 22.0 derived without LFE/LFE2',
      source_url: 'https://www.auro-3d.com/wp-content/uploads/2024/04/Auro-3D-in-multi-channel-WAV-White-Paper-20240419.pdf',
      sample_rate: sample_rate, bit_depth: 16, sample_format: 'signed PCM',
      amplitude: 0.5, frames_per_channel: frames, duration_seconds: frames.to_f / sample_rate,
      files: entries
    }
    File.write(File.join(dir, 'manifest.json'), JSON.pretty_generate(manifest) + "\n")
  end

  def self.write_wavex(path, mask, channels, sample_rate, frames)
    count = channels.length
    raise ArgumentError, 'Mask must match the channel count' unless mask.digits(2).sum == count

    block_align = count * 2
    # WAVEFORMATEXTENSIBLE with valid bits=16 and the PCM SubFormat GUID.
    fmt = [0xfffe, count, sample_rate, sample_rate * block_align, block_align, 16,
           22, 16, mask].pack('vvVVvvvvV')
    fmt += [1, 0, 0x0010, 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71].pack('VvvC8')
    data_size = frames * block_align
    File.open(path, 'wb') do |file|
      file.write('RIFF' + [60 + data_size].pack('V') + 'WAVE')
      file.write('fmt ' + [fmt.bytesize].pack('V') + fmt)
      file.write('data' + [data_size].pack('V'))
      # Write frame-aligned chunks rather than keeping the entire WAV in memory.
      (0...frames).step(1024) do |start|
        samples = (start...[start + 1024, frames].min).flat_map do |frame|
          channels.map do |channel|
            phase = 2 * Math::PI * channel.fetch(:frequency_hz) * frame / sample_rate
            (Math.sin(phase) * 0.5 * 32_767).round
          end
        end
        file.write(samples.pack('s<*'))
      end
    end
  end

  def self.data_dir(dir, file: nil)
    FileUtils.mkdir_p(dir)
    fn = dir
    fn = File.join(dir, file) if file
    fn
  end

  module SignalType
    SINE = :sine
    SQUARE = :square
    TRIANGLE = :triangle
    SAWTOOTH = :sawtooth
    TRAPEZIUM = :trapezium
    EXP = :exp
    WHITE_NOISE = :white_noise
    TPDF_NOISE = :tpdf_noise
    PINK_NOISE = :pink_noise
    BROWN_NOISE = :brown_noise
    PLUCK = :pluck
  end
  module SampleType
    SIGNED = :signed
    FLOAT = :float
  end

  class SoundFileBuilder
    # defaults to
    def initialize
      @bit_depth = 24
      @sample_type = SampleType::SIGNED
      @sample_rate = 48_000
      @signal = SignalType::SINE
      @frequency = 440
      @amplitude = 0.5
      @duration = 1
      @nr_channels = 1
    end

    def set(**kwargs)
      kwargs.each do |key, value|
        raise ArgumentError, "Unknown attribute #{key}" unless instance_variable_defined?("@#{key}")

        instance_variable_set("@#{key}", value)
      end
      self
    end

    def bit_depth(depth)
      @bit_depth = depth
      self
    end

    def sample_type(type)
      @sample_type = type
      self
    end

    def sample_rate(rate)
      @sample_rate = rate
      self
    end

    def signal(type)
      @signal = type
      self
    end

    def frequency(freq)
      @frequency = freq
      self
    end

    def amplitude(amp)
      @amplitude = amp
      self
    end

    def duration(time)
      @duration = time
      self
    end

    def nr_channels(channels)
      @nr_channels = channels
      self
    end

    def build(fn)
      args = %W[-n -r #{@sample_rate} -c #{@nr_channels} -b #{@bit_depth} -e #{@sample_type} #{fn} synth #{@duration}]
      @nr_channels.times do |i|
        args += %W[#{@signal} #{@frequency + i * 200}]
      end
      args += %W[vol #{@amplitude}]
      sox(args)
    end

    private

    def sox(args)
      puts ['sox', *args].join(' ')
      system('sox', *args, exception: true)
    end
  end

end

if $PROGRAM_NAME == __FILE__
  begin
    options = { kind: 'all', output_dir: TestData::DEFAULT_OUTPUT_DIR }
    OptionParser.new do |parser|
      parser.banner = 'Usage: ruby scripts/generate_test_data.rb [--kind all|basic|wavex] [--output-dir PATH]'
      parser.on('--kind KIND', %w[all basic wavex], 'Which fixtures to generate') { |kind| options[:kind] = kind }
      parser.on('--output-dir PATH', 'Output directory (default: repository data/)') { |path| options[:output_dir] = path }
    end.parse!
    raise ArgumentError, "Unexpected arguments: #{ARGV.join(' ')}" unless ARGV.empty?

    TestData.generate_basic(output_dir: options[:output_dir]) if %w[all basic].include?(options[:kind])
    TestData.generate_wavex(output_dir: options[:output_dir]) if %w[all wavex].include?(options[:kind])
  rescue ArgumentError, OptionParser::ParseError, SystemCallError, RuntimeError => e
    warn e.message
    exit 1
  end
end
