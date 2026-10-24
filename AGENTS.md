# Working on this fork (read first: Claude and Gemini)

This is Tootsby's fork of [slot](https://github.com/BrandonKowalski/slot), at
https://github.com/Tootsby/slot-Tootsby. Two AI tools work on it, taking turns:

- **Gemini / Claude in Google Antigravity** work directly in the local folder on Tootsby's PC
  (`C:\Claude projects\Gemini Projects\Slot Stuff`).
- **Claude in the Claude desktop app** works in its own cloud copy of the repo. It builds,
  tests, commits and pushes there. It cannot run git on the PC.

GitHub (`origin/main`) is the single source of truth. Each tool can only see the other's work
after it has been pushed to GitHub and pulled.

## Your job: tell Tootsby when to pull or push

Tootsby wants you to tell him, unprompted, when he needs to sync. Give him the exact commands
as a copy-paste block, and say which tool to run them in. Do this at these points:

### At the start of every session

**If you are in Antigravity (local folder):**
1. Run `git status` and `git fetch origin` yourself. If you can't run commands, ask Tootsby to.
2. If the local branch is behind `origin/main`, Claude probably pushed work from the cloud.
   Tell Tootsby before you edit anything:
   ```
   git pull
   ```
3. If there are uncommitted changes you didn't make, ask Tootsby whether to keep them
   before you pull.

**If you are Claude in the cloud:**
1. Run `git fetch origin` and work from the latest `origin/main`.
2. Ask Tootsby once whether Antigravity has anything uncommitted or unpushed. If it might, tell
   him to run this in Antigravity's terminal, in the Slot Stuff folder, before you start:
   ```
   git add -A
   git commit -m "WIP from Antigravity"
   git push
   ```

### Whenever you push to GitHub mid-session

Say so in one line ("Pushed to GitHub. Run `git pull` in the Slot Stuff folder before you
next use Antigravity.").

### At the end of a session, or when Tootsby says he's switching tools

- **Leaving Antigravity:** make sure the work is committed and pushed (`git push`). If it
  isn't, give him the commit and push commands above.
- **Leaving the cloud Claude session:** push everything, then tell him:
  ```
  git pull
  ```
  (Run it in the Slot Stuff folder, either in Antigravity's terminal or in PowerShell.)

### If a pull fails

- **"Your local changes would be overwritten":** tell him to run
  `git stash`, then `git pull`, then `git stash pop`, and help with any conflicts.
- **"Diverged" or a merge conflict:** both sides committed. Don't force-push. Resolve the merge
  (or have him run `git pull --no-rebase` and fix the conflicts with you), then push.
- Never run `git push --force` on `main` unless Tootsby explicitly asks.

## Project rules

- **Upstream comes first.** Base the fork on upstream's latest release. Keep the fork's
  changes only where they don't directly conflict with upstream. When upstream adds something
  that overlaps a fork feature, prefer upstream's version and re-apply the fork's tweaks on top.
- Remotes: `origin` (this fork), `upstream` (BrandonKowalski/slot). `pvaibhav` and
  `suprsomething` are where some borrowed features came from.
- **Versions and tags:** `v<upstream version>-tootsby.<n>`, e.g. `v1.9.0-tootsby.1`. Pushing a
  `v*` tag runs `.github/workflows/release.yml`, which builds the H700 card zip and publishes
  the GitHub release.
- **Before tagging:** add a CHANGELOG.md section for the new version at the top. Run
  `task test` (and `task lint` if possible). Commit, push, then tag and push the tag.
- Write commit messages in plain sentences, matching upstream's style.
- Target device: Anbernic RG SP (H700, aarch64) running BaseOS. Keep fork changes
  device-neutral unless Tootsby asks otherwise.
