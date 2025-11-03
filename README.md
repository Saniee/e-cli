# E-Cli

The command line tool for downloading Posts from e926.

it aims to be:
* Fast
* Verbose

What it can do:

- [x] Downloading Favourites of a user.
- [x] Downloading posts with specific Tags.
- [x] Multi-Threaded downloads.
- [x] Bulk Downloads.

What it can't do:

* Have a fully fledged UI.

# Usage

### Downloader Settings.

These can be set at the same time as you input commands.

#### Setting to use lower quality links.

`e-cli -l | --lower-quality {Command} {Command Arguments}`

#### Setting how many threads to use.

`e-cli -t | --num-threads {number of threads, default is 5, max is 10} {Command} {Command Arguments}`

#### Setting a different API source.

`e-cli -a | --api-source "anything.net" {Command} {Command Arguments}`

Anything that works like the e926 API.

## Downloading Favourites of a user with specific amount of posts.

`e-cli d-favourites {Username} -c {Number of Posts}`

## Downloading Random Favourites of a user

`e-cli d-favourites {Username} -r`

## Downloading posts with Tags

`e-cli d-tags {Tags}`