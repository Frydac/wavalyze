task :create_test_data do
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
            sample_rate: 44_100,
            nr_channels: 10,
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
        fn = data_dir(file: "#{kwargs[:signal]}_#{kwargs[:bit_depth]}_#{kwargs[:sample_type]}_#{kwargs[:sample_rate]}_#{kwargs[:nr_channels]}.wav")
        SoundFileBuilder.new.set(**kwargs).build(fn)
    end
end

def data_dir(*path, file: nil)
    dir = File.join('data', *path)
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
        command = "sox #{args.join(' ')}"
        puts command
        system(command)
    end
end
