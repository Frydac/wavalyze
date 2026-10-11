# frozen_string_literal: true

require 'json'
require 'date'
require 'digest'
require 'open3'
require 'optparse'

# Explicit development-time generation; Cargo consumes the committed Rust file.
class ChannelGenerator
  class Error < StandardError; end

  def initialize(root: File.expand_path('..', __dir__))
    @root = root
  end

  def run(check: false)
    outputs = generated_outputs
    stale = changed_paths(outputs)
    if check
      raise Error, "Channel definitions are stale (#{stale.join(', ')}); run rake channels:generate" unless stale.empty?

      puts 'Channel IDs and layouts match the local JSON definitions.'
    elsif stale.empty?
      puts 'Channel definitions are already up to date.'
    else
      write_outputs(outputs, stale)
    end
  end

  def import(source_dir)
    source_dir = File.expand_path(source_dir)
    paths = %w[id.json layout.json].to_h { |name| [name, File.join(source_dir, name)] }
    snapshots = paths.to_h { |name, path| ["data/channel/#{name}", File.binread(path)] }
    # Validate the exact bytes that will be copied; never reread the external files.
    outputs = generated_outputs(
      id_path: paths.fetch('id.json'), id_content: snapshots.fetch('data/channel/id.json'),
      layout_path: paths.fetch('layout.json'), layout_content: snapshots.fetch('data/channel/layout.json')
    )
    snapshot_changes = changed_paths(snapshots)
    provenance = 'data/channel/import.json'
    if !snapshot_changes.empty? || !File.exist?(File.join(@root, provenance))
      snapshots[provenance] = JSON.pretty_generate({
        'imported_on' => Date.today.iso8601,
        'source_directory' => source_dir,
        'sha256' => paths.keys.to_h { |name| [name, Digest::SHA256.hexdigest(snapshots.fetch("data/channel/#{name}"))] },
        'counts' => paths.keys.to_h { |name| [name, JSON.parse(snapshots.fetch("data/channel/#{name}")).size] }
      }) + "\n"
    end
    outputs = snapshots.merge(outputs)
    changed = changed_paths(outputs)
    if changed.empty?
      puts 'Channel snapshots and generated Rust are already up to date.'
    else
      write_outputs(outputs, changed)
      puts "Imported channel definitions from #{source_dir}; local supplemental masks were preserved."
    end
  end

  def layout_constants(ids, layout_path: File.join(@root, 'data/channel/layout.json'), layout_content: nil)
    constants = { 'NONE' => '0' }
    ids.each do |entry|
      add_constant(constants, channel_constant(entry.fetch('name')), "1 << Id::#{entry.fetch('name')} as u64")
    end
    known_ids = ids.to_h { |entry| [entry.fetch('name'), entry.fetch('value')] }
    layouts = read_array(layout_path, content: layout_content)
    layouts.sort_by { |entry| entry.is_a?(Hash) ? entry['name'].to_s : '' }.each do |entry|
      raise Error, 'Each layout must be an object' unless entry.is_a?(Hash)

      name = layout_constant(entry['name'])
      channels = entry['channels']
      unless channels.is_a?(Array) && channels.all? { |channel| channel.is_a?(String) }
        raise Error, "Layout #{entry['name']} must have an array of channel names"
      end
      raise Error, "Repeated channels in layout #{entry['name']}" unless channels.uniq == channels
      channels.each { |channel| raise Error, "Unknown channel #{channel} in layout #{entry['name']}" unless known_ids.key?(channel) }
      members = channels.sort_by { |channel| known_ids.fetch(channel) }.map { |channel| channel_constant(channel) }
      add_constant(constants, name, union_expression(members))
      synonyms = entry['synonyms']
      unless synonyms.is_a?(Array)
        raise Error, "Layout #{entry['name']} must have an array of synonyms"
      end
      unless entry['alternatives'].is_a?(Array) && entry['alternatives'].all? { |alternative| alternative.is_a?(String) }
        raise Error, "Layout #{entry['name']} must have an array of alternative names"
      end
      synonyms.map { |synonym| layout_constant(synonym) }.sort.each do |synonym|
        add_constant(constants, synonym, "Self::#{name}.bits()")
      end
    end

    supplements = read_array(File.join(@root, 'data/channel/supplemental.json'))
    definitions = {}
    supplements.each do |entry|
      raise Error, 'Each supplemental mask must be an object' unless entry.is_a?(Hash)

      name, members = entry.values_at('name', 'members')
      unless name.is_a?(String) && name.match?(/\A[A-Z][A-Z0-9_]*\z/)
        raise Error, "Invalid supplemental constant name: #{name.inspect}"
      end
      if constants.key?(name) || definitions.key?(name)
        raise Error, "Duplicate generated constant: #{name}"
      end
      unless members.is_a?(Array) && members.all? { |member| member.is_a?(String) } && members.uniq == members
        raise Error, "Supplemental mask #{name} must have distinct constant-name members"
      end
      definitions[name] = members
    end
    visiting = {}
    resolve = lambda do |name|
      return if constants.key?(name)
      raise Error, "Unknown supplemental mask member: #{name}" unless definitions.key?(name)
      raise Error, "Cycle in supplemental masks at #{name}" if visiting[name]

      visiting[name] = true
      definitions.fetch(name).sort.each { |member| resolve.call(member) }
      add_constant(constants, name, union_expression(definitions.fetch(name).sort))
      visiting.delete(name)
    end
    definitions.keys.sort.each { |name| resolve.call(name) }
    constants
  end

  def render_layouts(constants, ids, layouts)
    definitions = constants.map { |name, expression| "        const #{name} = #{expression};" }.join("\n")
    values = ids.to_h { |entry| [entry.fetch('name'), entry.fetch('value')] }
    groups = layouts.sort_by { |entry| entry.fetch('name') }.group_by do |entry|
      entry.fetch('channels').sum { |channel| 1 << values.fetch(channel) }
    end
    names = groups.sort.map do |mask, entries|
      labels = entries.flat_map { |entry| [entry.fetch('name')] + entry.fetch('synonyms').sort }
      "            0x#{mask.to_s(16)} => &[#{labels.map { |label| rust_string(label) }.join(', ')}],"
    end.join("\n")
    catalog = groups.keys.sort.map { |mask| "                  Layout::from_bits_retain(0x#{mask.to_s(16)})," }.join("\n")
    <<~RUST
      // Generated from data/channel/{id,layout,supplemental}.json by scripts/generate_channels.rb.
      // Do not edit; run rake channels:generate instead.

      use bitflags::bitflags;
      use super::Id;

      bitflags! {
          #[derive(Debug, Clone, PartialEq, Eq)]
          pub struct Layout: u64 {
      #{definitions}
          }
      }

      impl Layout {
          /// Distinct layouts from the imported catalog, ordered by mask.
          pub fn catalog() -> &'static [Self] {
              const CATALOG: &[Layout] = &[
      #{catalog}
              ];
              CATALOG
          }

          /// Catalog names and synonyms for an exact mask match, or an empty slice.
          pub fn names(&self) -> &'static [&'static str] {
              match self.bits() {
      #{names}
                  _ => &[],
              }
          }
      }

      #[cfg(test)]
      #[path = "layout_tests.rs"]
      mod tests;
    RUST
  end

  def load_ids(path, content: nil)
    ids = JSON.parse(content || File.binread(path))
    raise Error, 'Channel IDs must be a nonempty JSON array' unless ids.is_a?(Array) && !ids.empty?

    names = {}
    values = {}
    ids.each_with_index do |entry, index|
      raise Error, "Channel ID entry #{index} must be an object" unless entry.is_a?(Hash)

      name, value, short = entry.values_at('name', 'value', 'short')
      unless name.is_a?(String) && name.match?(/\A[A-Z][A-Za-z0-9_]*\z/) && name != 'Self'
        raise Error, "Invalid Rust channel ID name at entry #{index}: #{name.inspect}"
      end
      unless value.is_a?(Integer) && value.between?(0, 63)
        raise Error, "Channel ID #{name} must have an integer value in 0..63 for the u64 layout mask"
      end
      raise Error, "Channel ID #{name} must have a nonempty short name" unless short.is_a?(String) && !short.empty?
      raise Error, "Duplicate channel ID name: #{name}" if names.key?(name)
      raise Error, "Duplicate channel ID value #{value}: #{values[value]} and #{name}" if values.key?(value)

      names[name] = true
      values[value] = name
    end
    ids.sort_by { |entry| entry.fetch('value') }
  rescue JSON::ParserError => e
    raise Error, "Invalid JSON in #{path}: #{e.message}"
  end

  def render_ids(ids)
    variants = ids.map { |entry| "    #{entry.fetch('name')} = #{entry.fetch('value')}," }.join("\n")
    short_names = ids.map do |entry|
      "            Id::#{entry.fetch('name')} => #{rust_string(entry.fetch('short'))},"
    end.join("\n")
    <<~RUST
      // Generated from data/channel/id.json by scripts/generate_channels.rb.
      // Do not edit; run rake channels:generate instead.

      use strum_macros::{AsRefStr, EnumIter, EnumString};

      #[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, EnumString, AsRefStr)]
      #[repr(u32)]
      pub enum Id {
      #{variants}
      }

      impl Id {
          pub fn short_name(&self) -> &'static str {
              match self {
      #{short_names}
              }
          }

          /// Long name: same as enum variant name.
          pub fn long_name(&self) -> &str {
              self.as_ref()
          }
      }

      #[cfg(test)]
      #[path = "id_tests.rs"]
      mod tests;
    RUST
  end

  private

  def generated_outputs(id_path: File.join(@root, 'data/channel/id.json'), id_content: nil,
                        layout_path: File.join(@root, 'data/channel/layout.json'), layout_content: nil)
    ids = load_ids(id_path, content: id_content)
    layout_content ||= File.binread(layout_path)
    constants = layout_constants(ids, layout_path: layout_path, layout_content: layout_content)
    layouts = read_array(layout_path, content: layout_content)
    {
      'src/audio/channel/id.rs' => format_rust(render_ids(ids)),
      'src/audio/channel/layout.rs' => format_rust(render_layouts(constants, ids, layouts))
    }
  end

  def changed_paths(outputs)
    outputs.keys.select do |path|
      target = File.join(@root, path)
      !File.exist?(target) || File.binread(target) != outputs.fetch(path).b
    end
  end

  def write_outputs(outputs, paths)
    paths.each do |path|
      File.binwrite(File.join(@root, path), outputs.fetch(path))
      puts "Updated #{path}."
    end
  end

  def read_array(path, content: nil)
    entries = JSON.parse(content || File.binread(path))
    raise Error, "#{path} must contain a JSON array" unless entries.is_a?(Array)

    entries
  rescue JSON::ParserError => e
    raise Error, "Invalid JSON in #{path}: #{e.message}"
  end

  def channel_constant(name)
    name.gsub(/([A-Z]+)([A-Z][a-z])/, '\\1_\\2').gsub(/([a-z0-9])([A-Z])/, '\\1_\\2').upcase
  end

  def layout_constant(name)
    unless name.is_a?(String) && name.match?(/\A[A-Za-z0-9_.+-]+\z/) && name.match?(/[A-Za-z0-9]/)
      raise Error, "Invalid layout name: #{name.inspect}"
    end

    "LAYOUT_#{name.tr('.+-', '___').upcase}"
  end

  def add_constant(constants, name, expression)
    raise Error, "Duplicate generated constant: #{name}" if constants.key?(name)

    constants[name] = expression
  end

  def union_expression(members)
    members.empty? ? '0' : members.map { |name| "Self::#{name}.bits()" }.join(' | ')
  end

  def rust_string(value)
    escaped = value.each_char.map do |char|
      case char
      when '"' then '\\"'
      when '\\' then '\\\\'
      when "\n" then '\\n'
      when "\r" then '\\r'
      when "\t" then '\\t'
      else
        char.ord < 32 || char.ord == 127 ? "\\u{#{char.ord.to_s(16)}}" : char
      end
    end.join
    "\"#{escaped}\""
  end

  def format_rust(source)
    output, error, status = Open3.capture3(
      'rustfmt', '--edition', '2024', '--emit', 'stdout', '--config', 'skip_children=true', stdin_data: source
    )
    raise Error, "rustfmt failed: #{error.strip}" unless status.success?

    output
  rescue Errno::ENOENT
    raise Error, 'rustfmt is required for generation; install the rustfmt toolchain component'
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    options = { check: false }
    OptionParser.new do |parser|
      parser.banner = 'Usage: ruby scripts/generate_channels.rb [--check | --import SOURCE_DIR]'
      parser.on('--check', 'Compare generated IDs and layouts without writing files') { options[:check] = true }
      parser.on('--import SOURCE_DIR', 'Validate, copy JSON snapshots, and regenerate Rust') { |directory| options[:source_dir] = directory }
    end.parse!
    raise ChannelGenerator::Error, "Unexpected arguments: #{ARGV.join(' ')}" unless ARGV.empty?

    if options[:source_dir]
      raise ChannelGenerator::Error, '--check and --import cannot be combined' if options[:check]

      ChannelGenerator.new.import(options[:source_dir])
    else
      ChannelGenerator.new.run(**options)
    end
  rescue ChannelGenerator::Error, OptionParser::ParseError, SystemCallError => e
    warn e.message
    exit 1
  end
end
