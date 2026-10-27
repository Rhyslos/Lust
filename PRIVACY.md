# Privacy

Lust collects nothing about you. There are no accounts, no tracking and no statistics sent anywhere. This page says everything Lust does that touches the internet or keeps something on your computer.

## What goes over the internet

Only one thing, and only to GitHub, where Lust's releases are:

- **Checking for a newer Lust.** At most once a day, when you run, build or check a file in a terminal, Lust asks GitHub for its list of releases. `lust update` and `lust update check` ask too, whenever you run them. `lust update` then downloads the new `lust.exe` from GitHub.
- **What GitHub sees:** your computer's internet address (IP address), and that the request comes from `lust-update`. Lust sends nothing else: not your files, your code, your name or anything about your computer. What GitHub does with its own logs is covered by [GitHub's privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).
- **Turning it off:** `lust update off` stops the daily check. Setting the environment variable `LUST_NO_UPDATE_NOTES` stops it too. It never runs in CI, or when Lust's output goes to another program, like Ledger.

Your own programs can use the internet (`Web:get`, servers with `route`). What they send is up to the program, not Lust.

## What's kept on your computer

- **Lust itself**, where the installer put it: `%LOCALAPPDATA%\Programs\Lust`.
- **One small settings file** for updates: `%LOCALAPPDATA%\Lust\update.txt` (on Linux, `~/.config/lust/update.txt`). It holds whether the update note is on, when Lust last checked, and the newest version it saw. Nothing else.
- **Build files** for `lust build`, in `%LOCALAPPDATA%\Lust` (on Linux, `~/.cache/lust`), so the next build is quicker. They're made from your own code, and you can delete them at any time.

Uninstalling Lust removes the program. The settings file and build files can be deleted by hand; Lust doesn't need them.

## Changes

If Lust ever starts doing more than this, this page will say so first, in the same release.
