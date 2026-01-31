# Contributing to ASB

Thank you for your interest in contributing to Android Skin Builder!

## Development Setup

1. Fork and clone the repository
2. Install dependencies: `npm install`
3. Build the project: `npm run build`
4. Make your changes in the `src/` directory
5. Test your changes

## Project Structure

```
asb/
├── src/
│   ├── builder.ts        # Main builder class
│   ├── index.ts          # CLI interface
│   ├── types.ts          # TypeScript type definitions
│   └── utils/
│       ├── aapt2.ts      # aapt2 wrapper utility
│       ├── aar.ts        # AAR extraction utility
│       └── cache.ts      # Build cache management
├── bin/
│   └── asb.js            # CLI entry point
├── examples/
│   └── simple-skin/      # Example skin project
└── dist/                 # Compiled JavaScript output
```

## Making Changes

1. Create a new branch for your feature or bugfix
2. Write clean, readable code
3. Follow the existing code style
4. Update documentation if needed
5. Test your changes thoroughly

## Submitting Pull Requests

1. Push your changes to your fork
2. Create a pull request with a clear description
3. Link any relevant issues
4. Wait for review

## Code Style

- Use TypeScript
- Follow existing formatting conventions
- Use meaningful variable and function names
- Add comments for complex logic
- Keep functions focused and concise

## Testing

Before submitting a PR:

1. Build the project: `npm run build`
2. Test the CLI with the example project
3. Ensure no TypeScript errors

## Reporting Issues

When reporting issues, please include:

- Clear description of the problem
- Steps to reproduce
- Expected vs actual behavior
- Environment information (OS, Node version, etc.)
- Relevant error messages or logs

## Questions?

Feel free to open an issue for questions or discussions.

Thank you for contributing!
