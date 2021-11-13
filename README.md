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

## Usage

The basic usage is simple. Set how much money you put aside for "fun stuff", and
in what interval, ie `sq budget --amount 50 --interval 30`, signifying $50 set
aside every 30 days. `sq` prorates this over the month, ie the ratio matters,
and is calculated in second increments.

Once you have your budget set, add things you want to have for yourself. This is
a strict queue, so whatever you enter will be appended. For example, `sq add A
new screen.`. `sq` will then prompt you for how much that would be:

```bash
p/sq ╍ sq add A fancy thing.
What does this cost?:
250
Adding "A fancy thing." for $250 to the list.
p/sq ╍
```

Then, you wait. Ideally, you've put `sq` somewhere where you look at it
regularly - I added it to my `fish.config`. That way I see the output of `sq
status` every time I open a shell.

Once `sq` tells you that your current budget is higher than the next thing in
the purchasing queue, hit `sq buy`. This will mark the top item as bought, and
shuffle the next one up. Treat this like a chore, something to do mechanically.

Then, the joy lands when it arrives. Or so the theory.


## Installing/Building

It's a very standard cargo project. If you keep your random, one-off binaries in
`~/.local/bin` like I do, there's even a pre-made rake task: `rake release`.

Other than that:

1. `cargo build --release`
2. `mv ./target/release/sq /somehere/on/$PATH`

## Things to do

Future improvements:

- [ ] Enable easily re-ordering the queue
- [ ] Provide subcommand to set current amount available.
- [ ] Add subcommand that prints the path to the statefile
- [ ] Implement cron-based mode where crossing the purchase tqhreshold adds the
      purchase to omnifocus
- [ ] Add subcommand that installs either a launchctl timer (OSX) or a systemd
      timer. Maybe even a crontab if we're feeling oldschool.

Bugs:
- [ ] `set-budget`: Fix panic when calling without args.
- [ ] `buy`: Setting the price flag on invocation is useless because the link is
      opened when the command is invoked, so the price is seen too late to
      adjust. Change to a prompt after opening.
- [ ] Make sure proration works correctly by writing a bunch of tests.
  - [ ] Write test that sets budget to $1/second in a day, then sleeps for one
        second, then check amount.

## License

Should this be ever needed, the project is explicitly licensed under the
GPLv3. Refer to the `LICENSE` file for more information.
