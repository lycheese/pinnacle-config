"""Sorts the capture inbox with the local model (Ollama). Run through triage.sh.

Each capture becomes one of:
  todo     moved to todo.md with a "- [ ] <task>" line the model writes
  archive  moved to archive.md (ideas, references, things already done)
  keep     stays in the inbox (unclear; you decide)

The inbox is backed up to the state folder first. Nothing changes if the model's answer cannot
be read. Prints a one-line summary.

Environment: INBOX, NOTES_DIR, BACKUP_DIR, OLLAMA_URL, MODEL.
"""

import datetime
import json
import os
import re
import shutil
import sys
import urllib.request

BATCH = 15

SYSTEM = """You triage a personal inbox of quick captures (thoughts, notes, links, snippets).
For every capture decide one action:
- "todo": something the person should do. Write "task" as a short imperative line.
- "archive": an idea, reference, fact or link worth keeping but with nothing to do now.
- "keep": unclear, or needs the person's judgement.
Answer for every id. Write tasks in the language of the capture."""

SCHEMA = {
    "type": "object",
    "properties": {
        "decisions": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "integer"},
                    "action": {"type": "string", "enum": ["todo", "archive", "keep"]},
                    "task": {"type": "string"},
                },
                "required": ["id", "action"],
            },
        }
    },
    "required": ["decisions"],
}


def split_inbox(text):
    """Returns (header, [entries]); each entry starts with its "## " line."""
    parts = re.split(r"(?m)^(?=## )", text)
    header = parts[0] if parts and not parts[0].startswith("## ") else ""
    entries = [p for p in parts if p.startswith("## ")]
    return header, entries


def ask(url, model, entries, first_id):
    listing = "\n\n".join(
        f"<capture id={first_id + i}>\n{e.strip()}\n</capture>" for i, e in enumerate(entries)
    )
    body = {
        "model": model,
        "stream": False,
        "think": False,
        "format": SCHEMA,
        "options": {"temperature": 0, "num_ctx": 16384},
        "messages": [
            {"role": "system", "content": SYSTEM},
            {"role": "user", "content": listing},
        ],
    }
    req = urllib.request.Request(
        url.rstrip("/") + "/api/chat",
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=600) as resp:
        reply = json.load(resp)
    return json.loads(reply["message"]["content"])["decisions"]


def main():
    inbox = os.environ["INBOX"]
    notes = os.environ["NOTES_DIR"]
    with open(inbox, encoding="utf-8") as f:
        text = f.read()
    header, entries = split_inbox(text)
    if not entries:
        print("Inbox is empty")
        return 0

    decisions = {}
    try:
        for start in range(0, len(entries), BATCH):
            for d in ask(os.environ["OLLAMA_URL"], os.environ["MODEL"],
                         entries[start:start + BATCH], start):
                decisions[d["id"]] = d
    except Exception as err:  # noqa: BLE001 - any failure means: change nothing
        print(f"Triage failed, inbox unchanged: {err}")
        return 1

    stamp = datetime.datetime.now().strftime("%Y-%m-%d_%H%M%S")
    os.makedirs(os.environ["BACKUP_DIR"], exist_ok=True)
    shutil.copy2(inbox, os.path.join(os.environ["BACKUP_DIR"], f"inbox-{stamp}.md"))

    kept, todos, archived = [], [], []
    for i, entry in enumerate(entries):
        d = decisions.get(i, {"action": "keep"})
        if d["action"] == "todo":
            task = (d.get("task") or entry.splitlines()[0][3:]).strip()
            heading, _, rest = entry.partition("\n")
            todos.append(f"{heading}\n\n- [ ] {task}\n\n{rest.strip()}\n\n")
        elif d["action"] == "archive":
            archived.append(entry.rstrip() + "\n\n")
        else:
            kept.append(entry)

    def append(name, title, blocks):
        if not blocks:
            return
        path = os.path.join(notes, name)
        new = not os.path.exists(path)
        with open(path, "a", encoding="utf-8") as f:
            if new:
                f.write(f"# {title}\n\n")
            f.writelines(blocks)

    append("todo.md", "Todo", todos)
    append("archive.md", "Archive", archived)
    with open(inbox + ".tmp", "w", encoding="utf-8") as f:
        f.write(header.rstrip() + "\n" + ("\n" + "".join(kept) if kept else ""))
    os.replace(inbox + ".tmp", inbox)

    print(f"{len(todos)} to todo.md, {len(archived)} archived, {len(kept)} kept")
    return 0


if __name__ == "__main__":
    sys.exit(main())
