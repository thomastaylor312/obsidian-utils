# Obsidian Utils

This passion project is meant to be a collection of Unix style CLI utilities to work with
[Obsidian](https://obsidian.md/) vaults (and, in many cases, plain markdown files as well). It tries
to follow the Unix philosophy of doing one thing and doing it well for each utility. The eventual
goal is to have a set of CLI tools that can replicate many of the features of Obsidian, but in a
more scriptable, CLI-first way. This also means they can be used by AI agents that have access to
the command line.

As a passion project, I primarily designed it around working with my personal vault, but I'm not
planning on it being that way forever. If people find these useful, I'll keep adding features and
working on supporting all the different things that Obsidian does.

Lastly, this isn't intended to compete with what Obsidian does in any way. I still use the UI daily.
This was born out of having something that wasn't an MCP server (that often required the API
plugin) with access to my notes. A local tool seemed to fit that job nicely and could be useful in
many other contexts.

## Installing

You can find tarballs (for Linux and Mac) and zip files (for Windows) on the
[releases page](https://github.com/thomastaylor312/obsidian-utils/releases/latest). Download the
appropriate one for your OS, then download and uncompress it. Then place any of the utilities you
want into somewhere on your `PATH` (like `/usr/local/bin`).

As the project matures, I'll work on adding other installation scripts and methods 

## Usage

Here is a quick and dirty example that selects all notes that have both of the specified tags and
then generates a link graph for just those files:

```terminal
$ obsidian-tags '/path/to/vault' --filter 'tag:technology,cli' | obsidian-links --vault-dir '/path/to/vault' -o json | jq
{
  "/path/to/vault/References/Aider.md": {
    "exists": true,
    "links": [
      "/path/to/vault/References/Claude Code.md",
      "/path/to/vault/References/Roo Code.md"
    ],
    "backlinks": []
  },
  "/path/to/vault/References/Claude Code.md": {
    "exists": true,
    "links": [],
    "backlinks": [
      "/path/to/vault/References/Aider.md"
    ]
  },
  "/path/to/vault/References/Ghostty.md": {
    "exists": true,
    "links": [
      "/path/to/vault/References/Warp"
    ],
    "backlinks": []
  },
  "/path/to/vault/References/OpenRouter.md": {
    "exists": false,
    "links": [],
    "backlinks": [
      "/path/to/vault/References/aichat.md"
    ]
  },
  "/path/to/vault/References/Roo Code.md": {
    "exists": false,
    "links": [],
    "backlinks": [
      "/path/to/vault/References/Aider.md"
    ]
  },
  "/path/to/vault/References/Warp": {
    "exists": false,
    "links": [],
    "backlinks": [
      "/path/to/vault/References/Ghostty.md"
    ]
  },
  "/path/to/vault/References/aichat.md": {
    "exists": true,
    "links": [
      "/path/to/vault/References/OpenRouter.md"
    ],
    "backlinks": []
  }
}
```

All tools (unless otherwise specified) either take a single positional argument that should be a
directory (generally the vault directory) or a list of files can be passed in over stdin. The
default output format is plain text, but it is recommended for more complex things you use `-o json`
for outputting more structured data

The help documentation for each tool is fairly detailed, but some examples are below:

**Get a list of all files that have any of the specified tags as JSON**

```terminal
obsidian-tags '/path/to/vault' --filter 'tag-any:technology,cli' -o json
```

**Get a full link graph for your vault**

```terminal
obsidian-links '/path/to/vault' -o json
```

**Get all backlinks for a specific file in your vault**

```terminal
obsidian-links '/path/to/vault' -o json | jq '."/path/to/vault/References/Tailscale.md".backlinks'
[
  "/path/to/vault/References/Caddy.md",
  "/path/to/vault/Technology/Homelab/Garage Server.md",
  "/path/to/vault/Technology/Homelab/New Homelab Cluster or Server.md"
]
```

**Get files that have both tags and open them in an editor**

```terminal
obsidian-tags '/path/to/vault' --filter 'tag:technology,cli' | tr '\n' '\0' | xargs -0 hx
```

## Caveats and Known Issues

I cannot stress enough that this a work in progress and mostly tested with my own vault. A few
things I know are missing:

- Testing with Windows paths
- WikiLinks (I use standard markdown links almost exclusively for max compatibility)
- Plugin-heavy vaults (and the various things they use and abuse Markdown with)
- "Inline" tags that aren't in frontmatter

If this tool gets interest from others, I would more than welcome the contributions and would add
more tests as those features are added 

## Roadmap

Here are some general ideas of what I want to work on next:

- `obsidian-bases`: A parser and renderer for bases (with both JSON and table support)
- `obsidian-templates`: Possibly written in Go to leverage expr-lang, but something that can take
  the text of a template on stdin and output the rendered template on stdout
- Benchmarks for the CLI using something like `hyperfine`
- Metadata/frontmatter queries
- Automatically reading default values for flags like `--link-style` from the vault's
  `app.json` file

I also had some crazy ideas that I'm not sure will ever see the light of day:

- Some sort of caching layer for the vault using sqlite (basically pipe in any supported JSON
  output and have it automagically cache in the DB)
- A terminal based graph viewer that takes the output from `obsidian-links` as input

