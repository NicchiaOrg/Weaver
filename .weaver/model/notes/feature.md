# Notes

The developer's own notes on the project, things he wants to remember: one Markdown sheet per project, with no pages, no folders and nothing else beside it. The Notes view of a project window shows the sheet formatted as he writes it, like Notion: a line he starts as Markdown, such as `# ` for a heading or `[ ] ` for a checklist item, turns at once into that element, and the Markdown itself never shows. Checklists come first: an item is ticked with a click and stays in its place, struck through. He can also paste images into the sheet or drop image files on it; they are copied next to the sheet and shown where he put them.

The sheet and its images live in a notes folder inside `.weaver` that stays on the developer's computer: it is never versioned with the project, so another clone starts with no notes, and git ignores its files, so the File view shows the folder empty. It is created when the developer first writes something in the sheet or adds an image. Every change is written by itself, with no command to save.

The sheet is a plain Markdown file, which other programs can change too. Whatever the sheet cannot show formatted stays in it as plain text and is written back unchanged. An image of the sheet that is not in the notes folder shows only as a placeholder with its name or address: Weaver never reads images elsewhere nor fetches them from the web.

The notes are the developer's alone: Weaver never tells the agent about them and never sends them to it.
