# Spending Queue (sq)

This is a tiny thing that has sprung from me not being very good at being nice
to myself, specifically, that spending money on myself is hard, and that
justifying costs for that are hard. If I allocate money in a budget towards "fun
money", it will end up unused. I wrote more about this
[here](https://rambling.malignat.us/2020-06-18/decoupling-purchasing-and-joy).

`sq` is the tool I wrote for reminding me to spend that. I added a call to `sq`
to my `config.fish`, so the current state will be displayed to me whenever I
start up a shell.

In the future, I should probably also trigger Omnifocus Automation and the
likes, but for right now, this is good.


## Installing/Building

It's a very standard cargo project. If you keep your random, one-off binaries in
`~/.local/bin` like I do, there's even a pre-made rake task: `rake release`.

Other than that:

1. `cargo build --release`
2. `mv ./target/release/sq /somehere/on/$PATH`

## License

Should this be ever needed, the project is explicitly licensed under the
GPLv3. Refer to the `LICENSE` file for more information.
