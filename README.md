# sift - CLI RSS feed reader with recommender algorithm

sift is an RSS feed reader that

- fetches the full-text content in Markdown,
- lets you like and dislike articles, and
- learns from the liked and disliked articles to evaluate content based on how likely you are to like them.

It outputs the results as JSON or TSV, meaning that a TUI can be built with scripts hooking it into tools like fzf, though an optional TUI feature is on the roadmap.

## Feeds

Feeds are defined as an entry in `feeds.toml`, for example:

```toml
[[feeds]]
url = "https://lobste.rs/srs"
name = "Lobste.rs"              # optionally define a display name for the feed; otherwise, this defaults to the name provided by the feed
tags = ["tech", "programming"]
```

## Tags

Tags are used for

- filtering, and
- scoring bias.

### Filtering

The entries being displayed can be restricted to only a specific set of tags, for example:

```sh
sift show --tags programming
```

### Scoring

An entry marked as `liked` or `disliked` affects the score of another entry more the more tags they share with the entry. The weight of an entry $x$ with respect to $y$ is given by

$$
\text{similarity score} \times \sum_{i=0}^{|\text{tags}(x,y)|} \text{tag weight}[i]
$$

where

- $\text{tags}(x,y)$ is the set of tags shared between $x$ and $y$, and
- $\text{tag weigt}[i]$ is how much the tag $i$ weigh. By default, this is set to 1, but this can be configured in `tags.toml`, like so:

```toml
[[tags]]
name = "programming"
weight = 2

```
