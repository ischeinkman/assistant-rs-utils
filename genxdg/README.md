# GenXDG

This utility outputs a `TOML` file for `assistant-rs` which contains a mode tree for opening applications and actions parsed from 
Desktop Entries found via the XDG spec. The tree defines a single root-level message, `open`, that moves to the central `open app` mode.
From there, each `.desktop` file is given its own command, parsed from the English `Name` attribute. For Desktop Entries without any 
Desktop Actions defined, this command just runs the `Exec` attribute. For entries with Desktop Actions defined, a new submode is defined 
with a default action (IE, a command with a blank `message` field) equal to the `Exec` attribute, and other commands defined based 
on each Desktop Action's own `Name` and `Exec` attributes. 