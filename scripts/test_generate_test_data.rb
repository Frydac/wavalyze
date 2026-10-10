# frozen_string_literal: true

require 'minitest/autorun'
require 'tmpdir'
require 'digest'
require_relative 'generate_test_data'

class TestDataGeneratorTest < Minitest::Test
  def setup
    @dir = Dir.mktmpdir('wavalyze test content ')
    capture_io { TestData.generate_wavex(output_dir: @dir, duration: 0.03) }
    @wavex = File.join(@dir, 'wavex')
  end

  def teardown
    FileUtils.remove_entry(@dir)
  end

  def test_actual_headers_have_explicit_masks_and_valid_pcm_extensible_format
    {
      'stereo_2_0' => [0x000003, 2], 'quad_4_0' => [0x000603, 4],
      'surround_5_1' => [0x00060f, 6], 'auro_9_1' => [0x02d60f, 10],
      'auro_13_1' => [0x02fe3f, 14], 'nhk_22_0' => [0x7ffff7, 22],
      'nhk_22_2' => [0xffffff, 24]
    }.each do |stem, (mask, channels)|
      bytes = File.binread(File.join(@wavex, "wavex_#{stem}.wav"))
      assert_equal 'RIFF', bytes[0, 4]
      assert_equal bytes.bytesize - 8, bytes[4, 4].unpack1('V')
      assert_equal 'WAVE', bytes[8, 4]
      assert_equal 'fmt ', bytes[12, 4]
      assert_equal 40, bytes[16, 4].unpack1('V')
      assert_equal [0xfffe, channels, 48_000, 48_000 * channels * 2,
                    channels * 2, 16, 22, 16, mask], bytes[20, 24].unpack('vvVVvvvvV')
      # PCM GUID {00000001-0000-0010-8000-00AA00389B71} in little-endian WAV form.
      assert_equal '0100000000001000800000aa00389b71', bytes[44, 16].unpack1('H*')
      assert_equal 'data', bytes[60, 4]
      assert_equal 1440 * channels * 2, bytes[64, 4].unpack1('V')
      assert_equal 68 + 1440 * channels * 2, bytes.bytesize
    end
  end

  def test_manifest_lists_literal_pdf_nhk_order_and_masks
    manifest = JSON.parse(File.read(File.join(@wavex, 'manifest.json')))
    assert_equal 1440, manifest.fetch('frames_per_channel')
    assert_equal 7, manifest.fetch('files').size
    {
      'NHK_22.2' => ['0xFFFFFF', %w[L R C LFE Lb Rb LC RC Cs Ls Rs T HL HC HR HLs HCs HRs HLb HRb BL BC BR LFE2]],
      'NHK_22.0' => ['0x7FFFF7', %w[L R C Lb Rb LC RC Cs Ls Rs T HL HC HR HLs HCs HRs HLb HRb BL BC BR]]
    }.each do |layout, (mask, ids)|
      entry = manifest.fetch('files').find { |file| file.fetch('layout') == layout }
      assert_equal mask, entry.fetch('channel_mask')
      assert_equal ids, entry.fetch('channels').map { |channel| channel.fetch('id') }
      assert_equal (0...ids.length).to_a, entry.fetch('channels').map { |channel| channel.fetch('index') }
    end
  end

  def test_payload_contains_distinct_tones_in_serialized_channel_order
    samples = File.binread(File.join(@wavex, 'wavex_nhk_22_2.wav'))[68..].unpack('s<*')
    assert_equal [0] * 24, samples.first(24)
    assert_equal 24, samples[24, 24].uniq.length
    # Include both sides of the writer's 1024-frame chunk boundary.
    [1, 17, 127, 1023, 1024, 1439].each do |frame|
      24.times do |channel|
        frequency = 440 + channel * 200
        expected = Math.sin(2 * Math::PI * frequency * frame / 48_000) * 16_383.5
        assert_in_delta expected, samples[frame * 24 + channel], 0.5
      end
    end
  end

  def test_regeneration_produces_identical_bytes
    files = Dir[File.join(@wavex, '*')]
    digests = files.to_h { |path| [path, Digest::SHA256.file(path).hexdigest] }
    capture_io { TestData.generate_wavex(output_dir: @dir, duration: 0.03) }
    assert_equal digests, files.to_h { |path| [path, Digest::SHA256.file(path).hexdigest] }
  end
end
