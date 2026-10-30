#!/bin/bash
# Claude Code hook: sends the state of each session to the NumCal status screen (F3 mode) over USB serial.
# Without a connected NumCal it exits at once, and the serial write runs detached with a 1 second
# limit, so Claude Code never waits on the device.

port=/dev/cu.usbmodem123456783
[ -e "$port" ] || exit 0

line=$(jq -r '
    (.session_id // "" | .[0:8]) as $session
    | (.cwd // "" | split("/") | last) as $project
    | if .hook_event_name == "UserPromptSubmit" then ["busy", $project, "thinking"]
      elif .hook_event_name == "PreToolUse" and .tool_name == "AskUserQuestion" then ["wait", $project, "question"]
      elif .hook_event_name == "PreToolUse" and .tool_name == "ExitPlanMode" then ["wait", $project, "plan review"]
      elif .hook_event_name == "PreToolUse" then ["busy", $project, .tool_name]
      elif .hook_event_name == "PostToolUse" or .hook_event_name == "PostToolUseFailure" then ["busy", $project, "thinking"]
      elif .hook_event_name == "PermissionRequest" then ["wait", $project, "allow " + .tool_name + "?"]
      elif .hook_event_name == "Elicitation" then ["wait", $project, "question"]
      elif .hook_event_name == "Stop" then ["done", $project, ""]
      elif .hook_event_name == "Notification" and .notification_type == "idle_prompt" then ["done", $project, "idle"]
      elif .hook_event_name == "StopFailure" then ["done", $project, "failed"]
      elif .hook_event_name == "SessionStart" then ["ready", $project, (.source // "")]
      elif .hook_event_name == "SessionEnd" then ["off", $project, ""]
      else empty end
    | [$session] + . | join("\t")
' 2>/dev/null)

[ -n "$line" ] || exit 0

(perl -e 'alarm 1; open(my $tty, ">", $ARGV[0]) or exit; print $tty "$ARGV[1]\n";' "$port" "$line" &) >/dev/null 2>&1

exit 0
