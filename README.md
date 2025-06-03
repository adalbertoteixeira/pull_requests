# Pull Requests

Tool to automate the creation of PRs for JavaScript repositories.

## Highlights
- When a pre-commit hook fails, the next run automatically suggests the previously entered commit
message;
- Helps build a Github PR template and automatically check tasks.
- Extracts id and name from the branch name;
- Extracts type (and, in the future, scope) from the changed files;
- Output the pre-build pr template by passing `-r`.


## Install

### Install from npm (Recommended)

Install globally:
```bash
npm install -g @pull-requests/cli
```

Or install as a dev dependency in your project:
```bash
npm install --save-dev @pull-requests/cli
```

Then add a script to your `package.json`:
```json
{
  "scripts": {
    "commit": "pull-requests"
  }
}
```

The package includes pre-built binaries for:
- macOS (x64, arm64)
- Linux (x64)
- Windows (x64)

### Download binary
Navigate to the releases tab and download the latest `pull_requests` file for your platform.

### Build from source
Clone the repository and run

```bash
cargo build --release
```

For npm package maintainers, build all platforms:
```bash
npm run build
```

Add `.pull_requests` to your `.gitignore`.


## Usage

![demo](static_files/first.gif)

After the binary and the install command is set up, just run just run `[yarn | npm run | ...] commit`;

You can also trigger the tool manually using `./scripts/pull_requests`.

You can use flags in the command. The most useful one is the custom commit message (`-m "..."`). 

If you are committing several PRs to the same branch then type, prefix and id shouldn't change or will be
extracted automatically, but the commit message will probably be different every time.


The first time the tool is run an editor setup prompt will appear. For now only terminal based
`$EDITOR`s are mentioned, plus how to install VSCode usage.


### Get help

Help is available at any time by running
```{sh}
yarn commit -h
```
![help](static_files/help.gif)


## @TODO
- [ ] add tests;
- [ ] add support for scopes;
- [ ] cleanup old commit message files;
- [ ] add support for other languages;
- [x] install the library using yarn/npm;
- [ ] support for different systems (currently only macOS is tested);
- [ ] allow usage in Github Actions to automatically update the fields like labels based on scopes,
etc).
- [ ] allow using a config file at the repository level;
- [ ] automate adding`.pull_requests` to the `.gitignore` file;
- [ ] allow reuse of existing PR template (changing fields if necessary).
