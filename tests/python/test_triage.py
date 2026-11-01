"""Tests for scripts/capture/triage.py. Run: python3 -m unittest discover -s tests/python"""

import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts" / "capture"))
import triage  # noqa: E402

INBOX = """# Inbox

## 2026-11-01 10:00 #health

Call the dentist

## 2026-11-01 11:00

https://example.com/nice-article

## 2026-11-01 12:00

hmm
"""


class SplitInbox(unittest.TestCase):
    def test_header_and_entries(self):
        header, entries = triage.split_inbox(INBOX)
        self.assertEqual(header, "# Inbox\n\n")
        self.assertEqual(len(entries), 3)
        self.assertTrue(entries[0].startswith("## 2026-11-01 10:00 #health\n"))

    def test_no_entries(self):
        self.assertEqual(triage.split_inbox("# Inbox\n"), ("# Inbox\n", []))


class Main(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.notes = Path(self.dir.name)
        self.inbox = self.notes / "inbox.md"
        self.inbox.write_text(INBOX, encoding="utf-8")
        env = {
            "INBOX": str(self.inbox),
            "NOTES_DIR": str(self.notes),
            "BACKUP_DIR": str(self.notes / "backups"),
            "OLLAMA_URL": "http://unused",
            "MODEL": "test",
        }
        patcher = mock.patch.dict(os.environ, env)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.addCleanup(self.dir.cleanup)

    def run_with(self, decisions):
        with mock.patch.object(triage, "ask", return_value=decisions), \
                mock.patch("sys.stdout"):
            return triage.main()

    def test_moves_entries_and_backs_up(self):
        status = self.run_with([
            {"id": 0, "action": "todo", "task": "Call the dentist"},
            {"id": 1, "action": "archive"},
            {"id": 2, "action": "keep"},
        ])
        self.assertEqual(status, 0)
        todo = (self.notes / "todo.md").read_text()
        self.assertIn("## 2026-11-01 10:00 #health\n\n- [ ] Call the dentist\n\nCall the dentist", todo)
        self.assertTrue(todo.startswith("# Todo\n"))
        self.assertIn("nice-article", (self.notes / "archive.md").read_text())
        self.assertEqual(self.inbox.read_text(), "# Inbox\n\n## 2026-11-01 12:00\n\nhmm\n")
        self.assertEqual(len(list((self.notes / "backups").iterdir())), 1)

    def test_entries_without_a_decision_stay(self):
        self.run_with([{"id": 1, "action": "archive"}])
        inbox = self.inbox.read_text()
        self.assertIn("Call the dentist", inbox)
        self.assertIn("hmm", inbox)
        self.assertNotIn("nice-article", inbox)

    def test_todo_without_task_uses_the_first_line_of_text(self):
        self.run_with([{"id": 0, "action": "todo"}])
        self.assertIn("- [ ] Call the dentist\n", (self.notes / "todo.md").read_text())

    def test_unknown_action_keeps_the_entry(self):
        self.run_with([{"id": 0, "action": "delete"}])
        self.assertEqual(self.inbox.read_text(), INBOX)
        self.assertFalse((self.notes / "todo.md").exists())

    def test_existing_todo_file_is_appended_to(self):
        (self.notes / "todo.md").write_text("# Todo\n\nold\n\n", encoding="utf-8")
        self.run_with([{"id": 0, "action": "todo", "task": "Call"}])
        todo = (self.notes / "todo.md").read_text()
        self.assertTrue(todo.startswith("# Todo\n\nold\n\n## 2026-11-01 10:00"))
        self.assertEqual(todo.count("# Todo"), 1)

    def test_empty_inbox_is_left_alone(self):
        self.inbox.write_text("# Inbox\n")
        self.assertEqual(self.run_with([]), 0)
        self.assertFalse((self.notes / "backups").exists())

    def test_empty_inbox_after_triage_keeps_the_title(self):
        self.run_with([{"id": i, "action": "archive"} for i in range(3)])
        self.assertEqual(self.inbox.read_text(), "# Inbox\n")

    def test_model_failure_changes_nothing(self):
        with mock.patch.object(triage, "ask", side_effect=ValueError("bad json")), \
                mock.patch("sys.stdout"):
            self.assertEqual(triage.main(), 1)
        self.assertEqual(self.inbox.read_text(), INBOX)
        self.assertFalse((self.notes / "todo.md").exists())
        self.assertFalse((self.notes / "backups").exists())

    def test_large_inbox_is_sent_in_batches(self):
        many = "# Inbox\n" + "".join(f"\n## 2026-11-01 10:{i:02d}\n\nnote {i}\n" for i in range(20))
        self.inbox.write_text(many)
        calls = []

        def fake_ask(url, model, entries, first_id):
            calls.append((first_id, len(entries)))
            return [{"id": first_id + i, "action": "keep"} for i in range(len(entries))]

        with mock.patch.object(triage, "ask", side_effect=fake_ask), mock.patch("sys.stdout"):
            triage.main()
        self.assertEqual(calls, [(0, triage.BATCH), (triage.BATCH, 20 - triage.BATCH)])


if __name__ == "__main__":
    unittest.main()
