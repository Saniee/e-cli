# E-Cli
The command line tool for downloading E-Posts from a certain site.

It aims to be fast, and have verbose downloading console print outs.

What it can do:
- [x] Downloading Favourites of a user.
- [x] Downloading a single post from the site.
- [x] Downloading of multiple posts from a generated json file via get-pages
- [x] Downloading multiple pages

# Usage

## Downloading Favourites of a user with specific amount of posts.
```
e-tools.exe download-favourites {Username} -c {Number of Posts}
```

## Downloading Random Favourites of a user
```
e-tools.exe download-favourites {Username} -r
```

## Downloading a single post with lower quality
### Note, this argument can be used with any of the commands the CLI has.
```
e-tools.exe --lower-quality download-post {Post Id}
```

## Downloading pages of posts using the get-pages subcommand
First we get the pages via the subcommand:
```
e-tools.exe get-pages {Tags} -c {Number of Pages}
```

Then the command is as simple as it can be.
```
e-tools.exe download-posts posts.json
```
posts.json was generated in the root dir when get-pages was ran