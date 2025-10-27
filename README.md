# E-Cli

![GitHub Downloads (all assets, latest release)](https://img.shields.io/github/downloads/Saniee/e-cli/latest/total)
![GitHub last commit](https://img.shields.io/github/last-commit/Saniee/e-cli)

The command line tool for downloading Posts from a certain site.

It aims to be fast, and have verbose downloading console print outs.

What it can do:

- [x] Downloading Favourites of a user.
- [x] Multi-threaded downloads.
- [ ] Bulk Downloads.

What it can't do:

- Have a fully fledged App UI.

# Usage

### Specifying settings for the downloader.

These can be set at the same time.

#### Setting to use lower quality links.

`e-cli -l | --lower-quality {Command} {Command Arguments}`

#### Setting how many threads to use.

`e-cli -t | --num-threads {number of threads, default is 5, max is 10} {Command} {Command Arguments}`

#### Setting a different API source.

`e-cli -a | --api-source "anything.net" {Command} {Command Arguments}`

## Downloading Favourites of a user with specific amount of posts.

`e-cli download-favourites {Username} -c {Number of Posts}`

## Downloading Random Favourites of a user

`e-cli download-favourites {Username} -r`
