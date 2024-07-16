# md_serve

This is a simple Markdown-to-HTML server build on axum.

## Intended Use Case

0. Install `pandoc`
1. Put Markdown files in a directory structure
2. Run server in the root directory
3. Open browser

## Features

- Cache generated HTML locally
- Recursive folder structures

## Future Features

In no particular order
- Various command line options (e.g. port number)
- Real Favicon support
- Passing arguments to `pandoc` backend
