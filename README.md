# stl2ascii

**stl2ascii** is a text-based viewer for 3D files in STL format; it was created primarily to support LLMs that lack vision capabilities when working with 3D models. ASCII STL 3D viewer - interactive TUI with stdio MCP support.

Loads **ASCII and binary** STL, renders an orthographic shaded view as characters in a bordered frame, with a one-line status bar (camera + mesh stats).

## Build

```powershell
cargo build --release
```

Binary: `target/release/stl2ascii` (on Windows: `target\release\stl2ascii.exe`)

## CLI

```text
stl2ascii [OPTIONS] [FILE]
stl2ascii mcp
```

| Argument / option | Description |
|-------------------|-------------|
| `FILE` | Path to ASCII or binary STL |
| `--preset <PRESET>` | `front` \| `back` \| `left` \| `right` \| `top` \| `bottom` \| `iso` |
| `--yaw <DEG>` | Camera yaw (degrees) |
| `--pitch <DEG>` | Camera pitch (degrees) |
| `--zoom <F>` | Zoom factor (`>0`; larger = closer). Default `1` |
| `--pan-x <F>` | Horizontal pan |
| `--pan-y <F>` | Vertical pan |
| `--shade <MODE>` | `density` (default) \| `lines` |
| `--snapshot` | Render once to stdout and exit (ASCII left in the terminal) |
| `mcp` | Run local MCP server on stdio (stdout = protocol) |
| `-h`, `--help` | Dense help (features, options, keys) |
| `-V`, `--version` | Version |

Default view without `--preset` / angles: **iso**, auto-fit zoom.

### Examples

Using the release binary on a sample mesh:

```powershell
.\target\release\stl2ascii.exe .\corpus\t3c2_R13.stl --preset iso --snapshot
```

```bash
./target/release/stl2ascii ./corpus/t3c2_R13.stl --preset iso --snapshot
```

Via Cargo:

```powershell
cargo run --release -- .\corpus\t3c2_R13.stl --preset iso --snapshot
cargo run --release -- .\corpus\tube_mount_M5_nut_v2.stl --shade lines --preset front
cargo run --release -- mcp
```

### Interactive keys

| Key | Action |
|-----|--------|
| ←→ / `h` `l` | Yaw |
| ↑↓ / `j` `k` | Pitch |
| `+` / `-` | Zoom |
| `w` `a` `s` `d` | Pan |
| `m` | Toggle shade (`density` ↔ `lines`) |
| `r` | Reset camera |
| `q` / Esc | Quit |

### Shade modes

- **density** — brightness ramp  `.:-=+*#%@`
- **lines** — line-only glyphs  `.'`-_/|

### Status bar

Filename, shade mode, yaw/pitch/zoom, triangle count, AABB size, surface area, signed volume.

## MCP (stdio)

```powershell
stl2ascii mcp
```

Stdout is the MCP JSON-RPC channel; put human logs on stderr only.

### Cursor / MCP host config

```json
{
  "mcpServers": {
    "stl2ascii": {
      "command": "./target/release/stl2ascii",
      "args": ["mcp"]
    }
  }
}
```

On Windows, point `command` at `./target/release/stl2ascii.exe` (or the absolute path to your built binary).

### Tools

| Tool | Args | Result |
|------|------|--------|
| `render_stl` | `path`, optional `preset`, `yaw`, `pitch`, `zoom`, `pan_x`, `pan_y`, `shade`, `width`, `height` | Bordered ASCII + status line |
| `mesh_stats` | `path` | JSON: triangles, AABB, area, volume, center |
| `list_presets` | — | Preset names with yaw/pitch |
| `list_shade_modes` | — | `density`, `lines` + descriptions |

Paths are local filesystem paths. Each call reloads the file (stateless).

## License

MIT.
