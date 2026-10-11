# frozen_string_literal: true

require 'minitest/autorun'
require 'tmpdir'
require 'fileutils'
require_relative 'generate_channels'

class ChannelGeneratorTest < Minitest::Test
  def setup
    @root = Dir.mktmpdir('wavalyze-channel-generator-')
    FileUtils.mkdir_p(File.join(@root, 'data/channel'))
    FileUtils.mkdir_p(File.join(@root, 'src/audio/channel'))
    @input = File.join(@root, 'data/channel/id.json')
    @output = File.join(@root, 'src/audio/channel/id.rs')
    @layout_input = File.join(@root, 'data/channel/layout.json')
    @supplemental_input = File.join(@root, 'data/channel/supplemental.json')
    @layout_output = File.join(@root, 'src/audio/channel/layout.rs')
    @generator = ChannelGenerator.new(root: @root)
    @ids = [
      { 'name' => 'Left', 'value' => 0, 'short' => 'L' },
      { 'name' => 'LeftCenter', 'value' => 18, 'short' => 'LC' }
    ]
    write_ids(@ids)
    @layouts = [
      { 'name' => '1.0', 'channels' => ['Left'], 'synonyms' => ['Single'], 'alternatives' => ['mono'] },
      { 'name' => 'TestMix', 'channels' => ['LeftCenter', 'Left'], 'synonyms' => [], 'alternatives' => [] }
    ]
    File.write(@layout_input, JSON.generate(@layouts))
    File.write(@supplemental_input, '[]')
  end

  def teardown
    FileUtils.remove_entry(@root)
  end

  def write_ids(ids)
    File.write(@input, JSON.generate(ids))
  end

  def run_generator(**options)
    capture_io { @generator.run(**options) }
  end

  def test_explicit_values_survive_reordering_and_additions
    run_generator
    original = File.read(@output)
    write_ids(@ids.reverse)
    run_generator
    assert_equal original, File.read(@output)
    assert_includes original, 'LeftCenter = 18'
    assert_includes original, 'Id::LeftCenter => "LC"'

    write_ids(@ids + [{ 'name' => 'NewChannel', 'value' => 63, 'short' => 'N' }])
    run_generator
    output = File.read(@output)
    assert_includes output, 'NewChannel = 63'
    assert_includes output, 'LeftCenter = 18'
  end

  def test_check_detects_missing_and_stale_output_without_writing
    assert_raises(ChannelGenerator::Error) { run_generator(check: true) }
    refute File.exist?(@output)
    run_generator
    original = File.read(@output)
    timestamp = File.mtime(@output)
    run_generator(check: true)
    assert_equal timestamp, File.mtime(@output)
    write_ids(@ids.map { |id| id.merge('short' => "updated_#{id['short']}") })
    error = assert_raises(ChannelGenerator::Error) { run_generator(check: true) }
    assert_includes error.message, 'stale'
    assert_equal original, File.read(@output)
    assert_equal timestamp, File.mtime(@output)
  end

  def test_regeneration_does_not_rewrite_unchanged_output
    run_generator
    timestamp = File.mtime(@output)
    run_generator
    assert_equal timestamp, File.mtime(@output)
  end

  def test_invalid_inputs_leave_existing_output_untouched
    run_generator
    original = File.read(@output)
    invalid_inputs = [
      {}, [], [nil],
      @ids + [@ids.first.merge('value' => 1)],
      @ids + [@ids.first.merge('name' => 'Other')],
      [{ 'name' => 'Bad-Name', 'value' => 0, 'short' => 'B' }],
      [{ 'name' => 'Self', 'value' => 0, 'short' => 'S' }],
      *[-1, 64, 1.5, '1', nil].map { |value| [@ids.first.merge('value' => value)] },
      *['', nil, 123].map { |short| [@ids.first.merge('short' => short)] }
    ]
    invalid_inputs.each do |input|
      write_ids(input)
      assert_raises(ChannelGenerator::Error, input.inspect) { run_generator }
      assert_equal original, File.read(@output)
    end
    File.write(@input, '{bad json')
    assert_raises(ChannelGenerator::Error) { run_generator }
    assert_equal original, File.read(@output)
  end

  def test_short_names_are_escaped_as_rust_strings
    write_ids([@ids.first.merge('short' => "\"\\\n\r\t\u0001é"), @ids.last])
    run_generator
    output = File.read(@output)
    expected = <<~'RUST'.strip
      Id::Left => "\"\\\n\r\t\u{1}é",
    RUST
    assert_includes output, expected
  end

  def test_layouts_synonyms_and_supplements_are_deterministic
    supplements = [
      { 'name' => 'GROUP', 'members' => ['LEFT_CENTER', 'LEFT'] },
      { 'name' => 'LEGACY_ALIAS', 'members' => ['GROUP'] }
    ]
    File.write(@supplemental_input, JSON.generate(supplements))
    run_generator
    original = File.read(@layout_output)
    assert_includes original, 'const LEFT_CENTER = 1 << Id::LeftCenter as u64;'
    assert_includes original, 'const LAYOUT_TESTMIX = Self::LEFT.bits() | Self::LEFT_CENTER.bits();'
    assert_includes original, 'const LAYOUT_SINGLE = Self::LAYOUT_1_0.bits();'
    assert_includes original, 'const LEGACY_ALIAS = Self::GROUP.bits();'
    refute_includes original, 'LAYOUT_MONO'
    assert_equal ['0x1', '0x40001'], original.scan(/Layout::from_bits_retain\((0x[0-9a-f]+)\)/).flatten

    File.write(@layout_input, JSON.generate(@layouts.reverse.map { |layout| layout.merge('channels' => layout['channels'].reverse) }))
    File.write(@supplemental_input, JSON.generate(supplements.reverse.map { |entry| entry.merge('members' => entry['members'].reverse) }))
    run_generator
    assert_equal original, File.read(@layout_output)
  end

  def test_layout_check_detects_changes_without_rewriting_either_file
    run_generator
    originals = [@output, @layout_output].to_h { |path| [path, [File.read(path), File.mtime(path)]] }
    File.write(@layout_input, JSON.generate([@layouts.first.merge('channels' => ['LeftCenter'])]))
    assert_raises(ChannelGenerator::Error) { run_generator(check: true) }
    originals.each { |path, original| assert_equal original, [File.read(path), File.mtime(path)] }
  end

  def test_invalid_layouts_do_not_partially_regenerate_ids
    run_generator
    originals = [@output, @layout_output].to_h { |path| [path, File.read(path)] }
    write_ids(@ids.map { |id| id.merge('short' => 'changed') })
    invalid_inputs = [
      {}, [nil],
      [@layouts.first.merge('channels' => ['Unknown'])],
      [@layouts.first.merge('channels' => ['Left', 'Left'])],
      [@layouts.first.merge('channels' => 'Left')],
      [@layouts.first.merge('name' => 'invalid name')],
      [@layouts.first.merge('synonyms' => ['1-0'])],
      [@layouts.first.merge('synonyms' => [123])],
      [@layouts.first.merge('synonyms' => nil)],
      [@layouts.first, @layouts.first.merge('name' => '1+0')],
      [@layouts.first, @layouts.first.merge('name' => 'Other', 'synonyms' => ['Single'])]
    ]
    invalid_inputs.each do |input|
      File.write(@layout_input, JSON.generate(input))
      assert_raises(ChannelGenerator::Error, input.inspect) { run_generator }
      originals.each { |path, original| assert_equal original, File.read(path) }
    end
    File.write(@layout_input, '{invalid')
    assert_raises(ChannelGenerator::Error) { run_generator }
    originals.each { |path, original| assert_equal original, File.read(path) }
  end

  def test_supplements_cannot_override_constants_or_contain_bad_references
    run_generator
    original = File.read(@layout_output)
    invalid_inputs = [
      [{ 'name' => 'LAYOUT_SINGLE', 'members' => ['LEFT_CENTER'] }],
      [{ 'name' => 'LEFT', 'members' => [] }],
      [{ 'name' => 'BAD', 'members' => ['MISSING'] }],
      [{ 'name' => 'BAD', 'members' => ['LEFT', 'LEFT'] }],
      [{ 'name' => 'bad', 'members' => [] }],
      [{ 'name' => 'LOOP', 'members' => ['OTHER'] }, { 'name' => 'OTHER', 'members' => ['LOOP'] }]
    ]
    invalid_inputs.each do |input|
      File.write(@supplemental_input, JSON.generate(input))
      assert_raises(ChannelGenerator::Error, input.inspect) { run_generator }
      assert_equal original, File.read(@layout_output)
    end
  end

  def test_different_layout_names_may_share_a_mask
    File.write(@layout_input, JSON.generate([@layouts.first, @layouts.first.merge('name' => 'Other', 'synonyms' => [])]))
    run_generator
    assert_includes File.read(@layout_output), 'const LAYOUT_OTHER = Self::LEFT.bits();'
    assert_includes File.read(@layout_output), '0x1 => &["1.0", "Single", "Other"],'
    assert_equal ['0x1'], File.read(@layout_output).scan(/Layout::from_bits_retain\((0x[0-9a-f]+)\)/).flatten
  end

  def source_directory
    directory = File.join(@root, 'workplace source')
    FileUtils.mkdir_p(directory)
    directory
  end

  def write_source(ids: @ids, layouts: @layouts)
    directory = source_directory
    File.binwrite(File.join(directory, 'id.json'), JSON.pretty_generate(ids) + "\n")
    File.binwrite(File.join(directory, 'layout.json'), JSON.pretty_generate(layouts) + "\n")
    directory
  end

  def local_state
    [@input, @layout_input, @supplemental_input, @output, @layout_output,
     File.join(@root, 'data/channel/import.json')].to_h do |path|
      [path, File.exist?(path) ? [File.binread(path), File.mtime(path)] : nil]
    end
  end

  def test_import_preserves_source_bytes_and_local_supplements
    supplements = JSON.generate([{ 'name' => 'LOCAL_GROUP', 'members' => ['LEFT_CENTER', 'LEFT'] }])
    File.write(@supplemental_input, supplements)
    ids = @ids + [{ 'name' => 'NewChannel', 'value' => 63, 'short' => 'N', 'future_field' => 'retained' }]
    layouts = @layouts + [{ 'name' => 'New', 'channels' => ['NewChannel'], 'synonyms' => [], 'alternatives' => ['new alternative'] }]
    directory = write_source(ids: ids, layouts: layouts)
    File.write(File.join(directory, 'supplemental.json'), 'invalid external supplement')
    capture_io { @generator.import(directory) }
    assert_equal File.binread(File.join(directory, 'id.json')), File.binread(@input)
    assert_equal File.binread(File.join(directory, 'layout.json')), File.binread(@layout_input)
    assert_equal supplements, File.read(@supplemental_input)
    assert_includes File.read(@output), 'NewChannel = 63'
    assert_includes File.read(@layout_output), 'const LAYOUT_NEW = Self::NEW_CHANNEL.bits();'
    assert_includes File.read(@layout_output), 'const LOCAL_GROUP = Self::LEFT.bits() | Self::LEFT_CENTER.bits();'
    provenance = JSON.parse(File.read(File.join(@root, 'data/channel/import.json')))
    assert_equal directory, provenance.fetch('source_directory')
    assert_equal 3, provenance.fetch('counts').fetch('id.json')
    assert_equal Digest::SHA256.file(@input).hexdigest, provenance.fetch('sha256').fetch('id.json')
    run_generator(check: true)
  end

  def test_importing_unchanged_inputs_does_not_rewrite_files_or_provenance
    directory = write_source
    capture_io { @generator.import(directory) }
    original = local_state
    capture_io { @generator.import(directory) }
    assert_equal original, local_state

    # An identical import also repairs stale Rust without replacing snapshots.
    File.write(@layout_output, 'stale')
    capture_io { @generator.import(directory) }
    original.each do |path, state|
      next if path == @layout_output

      assert_equal state, local_state.fetch(path)
    end
    assert_equal original.fetch(@layout_output).first, File.binread(@layout_output)
  end

  def test_invalid_imports_do_not_replace_any_local_files
    directory = write_source
    capture_io { @generator.import(directory) }
    original = local_state
    changed_ids = @ids.map { |id| id.merge('short' => 'changed') }
    for layouts in [
      [@layouts.first.merge('channels' => ['Unknown'])],
      [@layouts.first.merge('alternatives' => nil)],
      [@layouts.first.merge('alternatives' => [123])],
      [@layouts.first.merge('synonyms' => ['1-0'])]
    ]
      write_source(ids: changed_ids, layouts: layouts)
      assert_raises(ChannelGenerator::Error) { capture_io { @generator.import(directory) } }
      assert_equal original, local_state
    end
    write_source(ids: [@ids.first.merge('value' => 64)])
    assert_raises(ChannelGenerator::Error) { capture_io { @generator.import(directory) } }
    assert_equal original, local_state
    write_source(ids: changed_ids)
    File.write(File.join(directory, 'layout.json'), '{invalid json')
    assert_raises(ChannelGenerator::Error) { capture_io { @generator.import(directory) } }
    assert_equal original, local_state
    File.unlink(File.join(directory, 'layout.json'))
    assert_raises(Errno::ENOENT) { capture_io { @generator.import(directory) } }
    assert_equal original, local_state
  end

  def test_formatting_failure_does_not_replace_snapshots_or_generated_files
    directory = write_source
    capture_io { @generator.import(directory) }
    original = local_state
    write_source(ids: @ids.map { |id| id.merge('short' => 'changed') })
    formatter = @generator.method(:format_rust)
    failing_formatter = lambda do |source|
      raise ChannelGenerator::Error, 'fixture formatter failure' if source.include?('bitflags!')

      formatter.call(source)
    end
    @generator.stub(:format_rust, failing_formatter) do
      assert_raises(ChannelGenerator::Error) { capture_io { @generator.import(directory) } }
    end
    assert_equal original, local_state
  end
end
