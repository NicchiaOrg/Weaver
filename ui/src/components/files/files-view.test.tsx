import { act, createEvent, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { FileEntry } from "@/bindings/FileEntry";
import type { TreeChanged } from "@/bindings/TreeChanged";
import type { TreeUpdate } from "@/bindings/TreeUpdate";
import { FilesView } from "@/components/files/files-view";
import { ProjectWindow } from "@/components/project-window";
import { TooltipProvider } from "@/components/ui/tooltip";
import { call, on } from "@/lib/backend";

const { tabs, toast } = vi.hoisted(() => ({
  tabs: {
    openFile: vi.fn(),
    closeUnder: vi.fn(),
    moveUnder: vi.fn(),
    unsavedUnder: vi.fn((_path: string) => [] as string[]),
    unsavedPaths: [] as string[],
    lockedPaths: new Set<string>(),
    saveAll: vi.fn(async () => [] as { path: string; reason: string }[]),
    closeAll: vi.fn(),
  },
  toast: { error: vi.fn(), warning: vi.fn() },
}));

vi.mock("sonner", () => ({ toast }));
vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));
vi.mock("@/components/files/file-tabs-context", () => ({
  FileTabsProvider: ({ children }: { children: unknown }) => children,
  useFileTabs: () => tabs,
}));
vi.mock("@/components/files/file-tabs", () => ({ FileTabs: () => <p>Open a file from the tree.</p> }));

const PROJECT = "0123456789abcdef";
const dir = (path: string): FileEntry => ({ name: path.split("/").pop() ?? path, path, kind: "folder" });
const file = (path: string): FileEntry => ({ name: path.split("/").pop() ?? path, path, kind: "file" });

let generation: number;
let listings: Record<string, FileEntry[]>;
let push: (changed: TreeChanged) => void;
let calls: [string, Record<string, unknown>][];
let failWith: Record<string, string>;

const update = (...paths: string[]): TreeUpdate => ({
  generation: ++generation,
  folders: paths.map((path) => ({ path, entries: listings[path] ?? [] })),
  removed: [],
});

function backend(name: string, args: Record<string, unknown>): Promise<unknown> {
  calls.push([name, args]);
  if (failWith[name]) return Promise.reject({ code: "refused", message: failWith[name] });
  switch (name) {
    case "files_open_view":
      return Promise.resolve({ generation: ++generation, folders: [{ path: "", entries: listings[""] }] });
    case "files_expand":
      return Promise.resolve(update(args.path as string));
    case "files_collapse":
    case "files_leave":
      return Promise.resolve(null);
    case "files_folders":
      return Promise.resolve((listings[args.path as string] ?? []).filter((entry) => entry.kind === "folder"));
    case "files_create":
      return Promise.resolve({ path: `${args.parent ? `${args.parent}/` : ""}${args.name}`, update: update(args.parent as string) });
    case "files_rename": {
      const path = args.path as string;
      const parent = path.includes("/") ? path.slice(0, path.lastIndexOf("/") + 1) : "";
      return Promise.resolve({ path: `${parent}${args.name}`, update: update() });
    }
    case "files_move":
      return Promise.resolve({ path: `${args.target ? `${args.target}/` : ""}${(args.path as string).split("/").pop()}`, update: update() });
    case "files_delete":
      return Promise.resolve(update());
    case "project_status":
      return Promise.resolve({ state: "has-model" });
    default:
      return Promise.reject(new Error(`unexpected command ${name}`));
  }
}

beforeEach(() => {
  generation = 0;
  calls = [];
  failWith = {};
  listings = {
    "": [dir(".weaver"), dir("docs"), dir("src"), file("README.md"), file("notes.txt")],
    ".weaver": [file(".weaver/system.md")],
    docs: [file("docs/guide.md")],
    src: [dir("src/deep"), file("src/main.ts"), file("src/lib.ts")],
    "src/deep": [file("src/deep/leaf.ts")],
  };
  tabs.openFile.mockReset();
  tabs.closeUnder.mockReset();
  tabs.moveUnder.mockReset();
  tabs.unsavedUnder.mockReset().mockReturnValue([]);
  tabs.unsavedPaths = [];
  tabs.lockedPaths = new Set();
  tabs.saveAll.mockReset().mockResolvedValue([]);
  tabs.closeAll.mockReset();
  toast.error.mockReset();
  vi.mocked(call).mockReset().mockImplementation(((name: string, args: Record<string, unknown>) => backend(name, args)) as unknown as typeof call);
  vi.mocked(on).mockImplementation(((name: string, handler: (changed: TreeChanged) => void) => {
    if (name === "files-tree-changed") push = handler;
    return Promise.resolve(() => {});
  }) as unknown as typeof on);
});

const called = (name: string) => calls.filter(([n]) => n === name).map(([, args]) => args);

function renderView() {
  return render(
    <TooltipProvider>
      <ProjectWindow projectId={PROJECT} initialView="files" views={{ files: <FilesView />, board: <p>Board content</p> }} />
    </TooltipProvider>,
  );
}

const tree = () => screen.findByRole("tree", { name: "Project files" });
const item = (name: string) => screen.getByRole("treeitem", { name: new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`) });
const names = () => within(screen.getByRole("tree", { name: "Project files" })).getAllByRole("treeitem").map((row) => row.textContent);

/** Right click on a row (or the empty space of the tree) and the menu that opens. */
async function openMenu(target: HTMLElement) {
  fireEvent.contextMenu(target);
  return screen.findByRole("menu");
}

describe("the Files entry", () => {
  it("opens the view, reads only the root and shows folders first with every folder collapsed", async () => {
    renderView();
    await tree();
    expect(names()).toEqual([".weaver", "docs", "src", "README.md", "notes.txt"]);
    expect(called("files_open_view")).toEqual([{ projectId: PROJECT }]);
    expect(called("files_expand")).toEqual([]);
    const rows = within(screen.getByRole("tree")).getAllByRole("treeitem");
    expect(rows.map((row) => row.getAttribute("aria-expanded"))).toEqual(["false", "false", "false", null, null]);
    expect(rows.every((row) => row.getAttribute("aria-level") === "1")).toBe(true);
    expect(screen.getByText("Open a file from the tree.")).toBeInTheDocument();
  });

  it("starts with the view header across the tree and the tabs", async () => {
    renderView();
    const treeNode = await tree();
    const header = screen.getByRole("heading", { level: 1, name: "Files" }).closest("header")!;
    expect(header).toHaveClass("h-14", "border-b");
    expect(header.contains(treeNode)).toBe(false);
    expect(header.nextElementSibling).toContainElement(treeNode);
    expect(header.nextElementSibling).toContainElement(screen.getByText("Open a file from the tree."));
  });

  it("is a sidebar entry after Jobs, and choosing another entry closes the view and tells the backend", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    const entries = within(screen.getByRole("navigation", { name: "Views" }))
      .getAllByRole("button")
      .map((button) => button.textContent);
    expect(entries.slice(0, 5)).toEqual(["Board", "Chat", "Jobs", "Files", "Requirements"]);
    await user.click(screen.getByRole("button", { name: "Board" }));
    expect(screen.queryByRole("tree", { name: "Project files" })).not.toBeInTheDocument();
    expect(screen.getByText("Board content")).toBeInTheDocument();
    expect(tabs.closeAll).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(called("files_leave")).toEqual([{ projectId: PROJECT }]));
    // Coming back reads the root again and keeps no state of the view.
    await user.click(screen.getByRole("button", { name: "Files" }));
    await tree();
    expect(called("files_open_view")).toHaveLength(2);
    expect(called("files_leave")).toHaveLength(1);
    expect(called("files_open_view").length).toBeGreaterThan(called("files_leave").length - 1);
  });
});

describe("the tree with the keyboard and the mouse", () => {
  it("moves with Up and Down, expands with Right, goes to the first child with Right again and back to the parent with Left", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    act(() => item(".weaver").focus());
    await user.keyboard("{ArrowDown}{ArrowDown}");
    expect(item("src")).toHaveFocus();
    await user.keyboard("{ArrowRight}");
    await waitFor(() => expect(names()).toContain("main.ts"));
    expect(item("src")).toHaveAttribute("aria-expanded", "true");
    expect(called("files_expand")).toEqual([{ projectId: PROJECT, path: "src" }]);
    expect(names()).toEqual([".weaver", "docs", "src", "deep", "main.ts", "lib.ts", "README.md", "notes.txt"].map((n) => n));
    await user.keyboard("{ArrowRight}");
    expect(item("deep")).toHaveFocus();
    expect(item("deep")).toHaveAttribute("aria-level", "2");
    await user.keyboard("{ArrowLeft}");
    expect(item("src")).toHaveFocus();
    await user.keyboard("{ArrowLeft}");
    await waitFor(() => expect(item("src")).toHaveAttribute("aria-expanded", "false"));
    expect(called("files_collapse")).toEqual([{ projectId: PROJECT, path: "src" }]);
    expect(names()).not.toContain("main.ts");
    await user.keyboard("{ArrowUp}");
    expect(item("docs")).toHaveFocus();
    await user.keyboard("{End}");
    expect(item("notes.txt")).toHaveFocus();
    await user.keyboard("{Home}");
    expect(item(".weaver")).toHaveFocus();
  });

  it("opens a file in a tab with Enter and with a click, and toggles a folder the same way", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(item("README.md"));
    expect(tabs.openFile).toHaveBeenCalledWith("README.md");
    await user.keyboard("{ArrowDown}");
    expect(item("notes.txt")).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(tabs.openFile).toHaveBeenLastCalledWith("notes.txt");
    await user.click(item("docs"));
    await waitFor(() => expect(item("docs")).toHaveAttribute("aria-expanded", "true"));
    await user.click(item("docs"));
    await waitFor(() => expect(item("docs")).toHaveAttribute("aria-expanded", "false"));
    item("docs").focus();
    await user.keyboard("{Enter}");
    await waitFor(() => expect(item("docs")).toHaveAttribute("aria-expanded", "true"));
    expect(tabs.openFile).toHaveBeenCalledTimes(2);
  });

  it("marks the selected row and carries the position of each row", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(item("README.md"));
    expect(item("README.md")).toHaveAttribute("aria-selected", "true");
    expect(item("notes.txt")).toHaveAttribute("aria-selected", "false");
    expect(item("README.md")).toHaveAttribute("aria-posinset", "4");
    expect(item("README.md")).toHaveAttribute("aria-setsize", "5");
  });

  it("shows a lock with its name and tooltip on a file open in a tab whose lock is held", async () => {
    const user = userEvent.setup();
    tabs.lockedPaths = new Set(["README.md"]);
    renderView();
    await tree();
    const lock = within(item("README.md")).getByRole("img", { name: "The agent is editing this file" });
    expect(lock).toBeInTheDocument();
    expect(within(item("notes.txt")).queryByRole("img")).not.toBeInTheDocument();
    await user.hover(lock);
    expect((await screen.findAllByText("The agent is editing this file")).length).toBeGreaterThan(0);
  });

  it("applies what the backend says when a listed folder changes, and ignores an older answer", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(item("docs"));
    await waitFor(() => expect(names()).toContain("guide.md"));
    act(() =>
      push({ projectId: PROJECT, generation: 50, folders: [{ path: "docs", entries: [file("docs/guide.md"), file("docs/new.md")] }], removed: [] }),
    );
    expect(names()).toContain("new.md");
    act(() => push({ projectId: PROJECT, generation: 40, folders: [{ path: "docs", entries: [file("docs/old.md")] }], removed: [] }));
    expect(names()).toContain("new.md");
    expect(names()).not.toContain("old.md");
    act(() => push({ projectId: "ffffffffffffffff", generation: 99, folders: [{ path: "docs", entries: [] }], removed: [] }));
    expect(names()).toContain("new.md");
    act(() => push({ projectId: PROJECT, generation: 60, folders: [], removed: ["docs"] }));
    expect(names()).not.toContain("new.md");
    expect(item("docs")).toHaveAttribute("aria-expanded", "false");
  });
});

describe("the context menu", () => {
  it("offers New file, New folder, Rename, Move to… and Delete on a folder", async () => {
    renderView();
    await tree();
    const menu = await openMenu(item("docs"));
    expect(within(menu).getAllByRole("menuitem").map((entry) => entry.textContent)).toEqual(["New file", "New folder", "Rename", "Move to…", "Delete"]);
  });

  it("leaves New file and New folder out on a file", async () => {
    renderView();
    await tree();
    const menu = await openMenu(item("README.md"));
    expect(within(menu).getAllByRole("menuitem").map((entry) => entry.textContent)).toEqual(["Rename", "Move to…", "Delete"]);
  });

  it("offers only New file and New folder on the empty space, for the root", async () => {
    renderView();
    const view = await tree();
    const menu = await openMenu(view);
    expect(within(menu).getAllByRole("menuitem").map((entry) => entry.textContent)).toEqual(["New file", "New folder"]);
  });

  it("opens with Shift+F10 and with the Menu key on the selected row", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(item("README.md"));
    await user.keyboard("{Shift>}{F10}{/Shift}");
    expect(within(await screen.findByRole("menu")).getByRole("menuitem", { name: "Rename" })).toBeInTheDocument();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("menu")).not.toBeInTheDocument());
    expect(item("README.md")).toHaveFocus();
    await user.keyboard("{ContextMenu}");
    expect(await screen.findByRole("menu")).toBeInTheDocument();
  });

  it("shows the actions that would change .weaver disabled, with the reason as tooltip and description", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    const menu = await openMenu(item(".weaver"));
    for (const label of ["New file", "New folder", "Rename", "Move to…", "Delete"]) {
      const entry = within(menu).getByRole("menuitem", { name: label });
      expect(entry).toHaveAttribute("aria-disabled", "true");
      expect(entry).toHaveAccessibleDescription(".weaver is written by Weaver: read-only here.");
    }
    await user.click(within(menu).getByRole("menuitem", { name: "Delete" }));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    await user.hover(within(menu).getByRole("menuitem", { name: "Rename" }));
    expect((await screen.findAllByText(".weaver is written by Weaver: read-only here.")).length).toBeGreaterThan(0);
  });

  it("disables them for what is inside .weaver too, and F2 and Delete do nothing there", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(item(".weaver"));
    await waitFor(() => expect(names()).toContain("system.md"));
    const menu = await openMenu(item("system.md"));
    expect(within(menu).getByRole("menuitem", { name: "Rename" })).toHaveAttribute("aria-disabled", "true");
    await user.keyboard("{Escape}");
    item("system.md").focus();
    await user.keyboard("{F2}");
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    await user.keyboard("{Delete}");
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });
});

describe("naming an entry in the tree", () => {
  it("renames with F2: the field holds the name, selected without its extension; Enter confirms", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(item("README.md"));
    await user.keyboard("{F2}");
    const field = await screen.findByRole("textbox", { name: "New name for README.md" });
    expect(field).toHaveValue("README.md");
    expect(field).toHaveFocus();
    expect((field as HTMLInputElement).selectionStart).toBe(0);
    expect((field as HTMLInputElement).selectionEnd).toBe(6);
    await user.keyboard("GUIDE{Enter}");
    await waitFor(() => expect(called("files_rename")).toEqual([{ projectId: PROJECT, path: "README.md", name: "GUIDE.md" }]));
    expect(tabs.moveUnder).toHaveBeenCalledWith("README.md", "GUIDE.md");
    await waitFor(() => expect(screen.queryByRole("textbox")).not.toBeInTheDocument());
  });

  it("cancels with Esc and by leaving the field", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(item("README.md"));
    await user.keyboard("{F2}");
    await screen.findByRole("textbox");
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    await user.keyboard("{F2}");
    await screen.findByRole("textbox");
    await user.click(screen.getByRole("tree", { name: "Project files" }));
    await waitFor(() => expect(screen.queryByRole("textbox")).not.toBeInTheDocument());
    expect(called("files_rename")).toEqual([]);
  });

  it("does nothing for the same name, and keeps the field with a toast when the rename is refused", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(item("README.md"));
    await user.keyboard("{F2}{Enter}");
    expect(called("files_rename")).toEqual([]);
    await waitFor(() => expect(screen.queryByRole("textbox")).not.toBeInTheDocument());
    failWith.files_rename = 'A file or folder named "notes.txt" already exists here.';
    await user.keyboard("{F2}");
    await screen.findByRole("textbox");
    await user.keyboard("{Control>}a{/Control}notes.txt{Enter}");
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('A file or folder named "notes.txt" already exists here.'));
    expect(screen.getByRole("textbox")).toBeInTheDocument();
    expect(tabs.moveUnder).not.toHaveBeenCalled();
  });

  it("renames a folder with the menu and selects the whole name", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    const menu = await openMenu(item("docs"));
    await user.click(within(menu).getByRole("menuitem", { name: "Rename" }));
    const field = (await screen.findByRole("textbox", { name: "New name for docs" })) as HTMLInputElement;
    expect([field.selectionStart, field.selectionEnd]).toEqual([0, 4]);
    expect(field).toHaveFocus();
  });

  it("makes a file in a folder: the folder opens, a field shows in its first row, Enter creates it", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    const menu = await openMenu(item("docs"));
    await user.click(within(menu).getByRole("menuitem", { name: "New file" }));
    const field = await screen.findByRole("textbox", { name: "New file name" });
    expect(field).toHaveFocus();
    expect(field).toHaveValue("");
    const rows = within(screen.getByRole("tree")).getAllByRole("treeitem");
    const at = rows.findIndex((row) => within(row).queryByRole("textbox"));
    expect(rows[at - 1]).toBe(item("docs"));
    expect(rows[at]).toHaveAttribute("aria-level", "2");
    await user.keyboard("todo.md{Enter}");
    await waitFor(() => expect(called("files_create")).toEqual([{ projectId: PROJECT, parent: "docs", name: "todo.md", folder: false }]));
    await waitFor(() => expect(screen.queryByRole("textbox")).not.toBeInTheDocument());
  });

  it("makes a folder in the root from the empty space and cancels it with Esc", async () => {
    const user = userEvent.setup();
    renderView();
    const view = await tree();
    const menu = await openMenu(view);
    await user.click(within(menu).getByRole("menuitem", { name: "New folder" }));
    const field = await screen.findByRole("textbox", { name: "New folder name" });
    expect(within(screen.getByRole("tree")).getAllByRole("treeitem")[0]).toContainElement(field);
    await user.keyboard("assets{Enter}");
    await waitFor(() => expect(called("files_create")).toEqual([{ projectId: PROJECT, parent: "", name: "assets", folder: true }]));
    const again = await openMenu(view);
    await user.click(within(again).getByRole("menuitem", { name: "New file" }));
    await screen.findByRole("textbox", { name: "New file name" });
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(called("files_create")).toHaveLength(1);
  });

  it("shows the reason of a refused creation in a toast and keeps the field", async () => {
    const user = userEvent.setup();
    failWith.files_create = "The name cannot be empty.";
    renderView();
    const view = await tree();
    const menu = await openMenu(view);
    await user.click(within(menu).getByRole("menuitem", { name: "New file" }));
    await screen.findByRole("textbox", { name: "New file name" });
    await user.keyboard("{Enter}");
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith("The name cannot be empty."));
    expect(screen.getByRole("textbox", { name: "New file name" })).toBeInTheDocument();
  });
});

describe("deleting", () => {
  it("asks first with Delete, the focus on Cancel; Move to Trash deletes and closes the tabs on it", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(item("notes.txt"));
    await user.keyboard("{Delete}");
    const dialog = await screen.findByRole("alertdialog", { name: 'Move "notes.txt" to the trash?' });
    expect(within(dialog).getByRole("button", { name: "Cancel" })).toHaveFocus();
    expect(within(dialog).queryByText(/unsaved changes/)).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Move to Trash" }));
    await waitFor(() => expect(called("files_delete")).toEqual([{ projectId: PROJECT, path: "notes.txt" }]));
    expect(tabs.closeUnder).toHaveBeenCalledWith("notes.txt");
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
  });

  it("says how many open files have unsaved changes that will be lost", async () => {
    const user = userEvent.setup();
    tabs.unsavedUnder.mockImplementation((path: string) => (path === "src" ? ["src/main.ts", "src/lib.ts"] : []));
    renderView();
    await tree();
    const menu = await openMenu(item("src"));
    await user.click(within(menu).getByRole("menuitem", { name: "Delete" }));
    const dialog = await screen.findByRole("alertdialog", { name: 'Move "src" to the trash?' });
    expect(within(dialog).getByText("2 open files have unsaved changes that will be lost.")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Move to Trash" })).toHaveClass("bg-destructive");
  });

  it("changes nothing with Cancel, and a refused deletion shows its reason", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(item("notes.txt"));
    await user.keyboard("{Delete}");
    await user.click(await screen.findByRole("button", { name: "Cancel" }));
    expect(called("files_delete")).toEqual([]);
    expect(tabs.closeUnder).not.toHaveBeenCalled();
    failWith.files_delete = 'Could not move "notes.txt" to the trash: there is no trash';
    await user.keyboard("{Delete}");
    await user.click(await screen.findByRole("button", { name: "Move to Trash" }));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Could not move "notes.txt" to the trash: there is no trash'));
    expect(tabs.closeUnder).not.toHaveBeenCalled();
    expect(names()).toContain("notes.txt");
  });
});

describe("moving", () => {
  const dataTransfer = () => ({ setData: vi.fn(), getData: vi.fn(), effectAllowed: "", dropEffect: "" });

  it("moves a row dropped on a folder, highlighting the folder under the pointer", async () => {
    renderView();
    await tree();
    const transfer = dataTransfer();
    fireEvent.dragStart(item("README.md"), { dataTransfer: transfer });
    fireEvent.dragOver(item("docs"), { dataTransfer: transfer });
    expect(item("docs")).toHaveAttribute("data-drop-target");
    fireEvent.dragOver(item("notes.txt"), { dataTransfer: transfer });
    expect(item("docs")).not.toHaveAttribute("data-drop-target");
    fireEvent.dragOver(item("docs"), { dataTransfer: transfer });
    fireEvent.drop(item("docs"), { dataTransfer: transfer });
    await waitFor(() => expect(called("files_move")).toEqual([{ projectId: PROJECT, path: "README.md", target: "docs" }]));
    expect(tabs.moveUnder).toHaveBeenCalledWith("README.md", "docs/README.md");
    expect(item("docs")).not.toHaveAttribute("data-drop-target");
  });

  it("moves a row dropped on the empty space of the tree to the root", async () => {
    const user = userEvent.setup();
    renderView();
    const view = await tree();
    await user.click(item("docs"));
    await waitFor(() => expect(names()).toContain("guide.md"));
    const transfer = dataTransfer();
    fireEvent.dragStart(item("guide.md"), { dataTransfer: transfer });
    fireEvent.dragOver(view, { dataTransfer: transfer });
    expect(view).toHaveAttribute("data-drop-target");
    fireEvent.drop(view, { dataTransfer: transfer });
    await waitFor(() => expect(called("files_move")).toEqual([{ projectId: PROJECT, path: "docs/guide.md", target: "" }]));
  });

  it("does nothing where the move can never be: its own folder, itself, inside itself, .weaver", async () => {
    const user = userEvent.setup();
    renderView();
    const view = await tree();
    await user.click(item("src"));
    await waitFor(() => expect(names()).toContain("deep"));
    const transfer = dataTransfer();
    // Onto the folder it is already in, and onto the root while in the root.
    fireEvent.dragStart(item("main.ts"), { dataTransfer: transfer });
    const sameFolder = createEvent.dragOver(item("src"), { dataTransfer: transfer });
    fireEvent(item("src"), sameFolder);
    expect(sameFolder.defaultPrevented).toBe(false);
    fireEvent.drop(item("src"), { dataTransfer: transfer });
    fireEvent.dragEnd(item("main.ts"), { dataTransfer: transfer });
    // A folder onto itself and into a folder inside it.
    fireEvent.dragStart(item("src"), { dataTransfer: transfer });
    fireEvent.drop(item("src"), { dataTransfer: transfer });
    fireEvent.drop(item("deep"), { dataTransfer: transfer });
    fireEvent.dragEnd(item("src"), { dataTransfer: transfer });
    // Into .weaver, and out of it: not even draggable.
    fireEvent.dragStart(item("README.md"), { dataTransfer: transfer });
    fireEvent.drop(item(".weaver"), { dataTransfer: transfer });
    fireEvent.dragEnd(item("README.md"), { dataTransfer: transfer });
    expect(item(".weaver")).toHaveAttribute("draggable", "false");
    // Onto a file, and the root for what is already in it.
    fireEvent.dragStart(item("docs"), { dataTransfer: transfer });
    fireEvent.drop(item("notes.txt"), { dataTransfer: transfer });
    fireEvent.drop(view, { dataTransfer: transfer });
    expect(called("files_move")).toEqual([]);
  });

  it("opens Move to… with the folders of the project read one at a time, and moves into the chosen one", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    const menu = await openMenu(item("README.md"));
    await user.click(within(menu).getByRole("menuitem", { name: "Move to…" }));
    const dialog = await screen.findByRole("dialog", { name: 'Move "README.md" to…' });
    const folders = within(dialog).getByRole("tree", { name: "Folders of the project" });
    await waitFor(() => expect(within(folders).getByRole("treeitem", { name: "src" })).toBeInTheDocument());
    expect(called("files_folders")).toEqual([{ projectId: PROJECT, path: "" }]);
    expect(within(folders).getAllByRole("treeitem").map((row) => row.textContent)).toEqual(["Project folder", ".weaver", "docs", "src"]);
    // Everything but where it is, and .weaver, can be chosen.
    expect(within(folders).getByRole("treeitem", { name: "Project folder" })).toHaveAttribute("aria-disabled", "true");
    expect(within(folders).getByRole("treeitem", { name: ".weaver" })).toHaveAttribute("aria-disabled", "true");
    const move = within(dialog).getByRole("button", { name: "Move" });
    expect(move).toHaveAttribute("aria-disabled", "true");
    // Expanding reads that folder only.
    const src = within(folders).getByRole("treeitem", { name: "src" });
    src.focus();
    await user.keyboard("{ArrowRight}");
    await waitFor(() => expect(within(folders).getByRole("treeitem", { name: "deep" })).toBeInTheDocument());
    expect(called("files_folders")).toEqual([
      { projectId: PROJECT, path: "" },
      { projectId: PROJECT, path: "src" },
    ]);
    await user.keyboard("{ArrowRight}{Enter}");
    expect(within(folders).getByRole("treeitem", { name: "deep" })).toHaveAttribute("aria-selected", "true");
    expect(move).not.toHaveAttribute("aria-disabled");
    await user.click(move);
    await waitFor(() => expect(called("files_move")).toEqual([{ projectId: PROJECT, path: "README.md", target: "src/deep" }]));
    expect(tabs.moveUnder).toHaveBeenCalledWith("README.md", "src/deep/README.md");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("cancels Move to… without moving, and keeps the dialog when the move is refused", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    const menu = await openMenu(item("notes.txt"));
    await user.click(within(menu).getByRole("menuitem", { name: "Move to…" }));
    const dialog = await screen.findByRole("dialog");
    await user.click(await within(dialog).findByRole("treeitem", { name: "docs" }));
    failWith.files_move = "The agent is editing this file.";
    await user.click(within(dialog).getByRole("button", { name: "Move" }));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith("The agent is editing this file."));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(tabs.moveUnder).not.toHaveBeenCalled();
  });
});

describe("leaving the view", () => {
  it("closes the tabs and shows the other view at once when nothing is unsaved", async () => {
    const user = userEvent.setup();
    renderView();
    await tree();
    await user.click(screen.getByRole("button", { name: "Board" }));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    expect(screen.getByText("Board content")).toBeInTheDocument();
    expect(tabs.closeAll).toHaveBeenCalledTimes(1);
    expect(tabs.saveAll).not.toHaveBeenCalled();
  });

  it("asks Save changes to <n> files?, listing the paths, with Cancel last and focused", async () => {
    const user = userEvent.setup();
    tabs.unsavedPaths = ["src/main.ts", "notes.txt"];
    renderView();
    await tree();
    await user.click(screen.getByRole("button", { name: "Board" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Save changes to 2 files?" });
    expect(within(dialog).getAllByRole("listitem").map((entry) => entry.textContent)).toEqual(["src/main.ts", "notes.txt"]);
    const buttons = within(dialog).getAllByRole("button");
    expect(buttons.map((button) => button.textContent)).toEqual(["Save all", "Discard all", "Cancel"]);
    expect(buttons[1]).toHaveClass("bg-destructive");
    expect(buttons[2]).toHaveFocus();
    expect(screen.queryByText("Board content")).not.toBeInTheDocument();
    expect(tabs.closeAll).not.toHaveBeenCalled();
  });

  it("keeps the File view as it is with Cancel", async () => {
    const user = userEvent.setup();
    tabs.unsavedPaths = ["notes.txt"];
    renderView();
    await tree();
    await user.click(screen.getByRole("button", { name: "Board" }));
    expect(await screen.findByRole("alertdialog", { name: "Save changes to 1 file?" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    expect(screen.getByRole("tree", { name: "Project files" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Files" })).toHaveAttribute("aria-current", "page");
    expect(tabs.closeAll).not.toHaveBeenCalled();
    expect(tabs.saveAll).not.toHaveBeenCalled();
    expect(called("files_leave")).toEqual([]);
  });

  it("closes every tab without saving with Discard all, then shows the other view", async () => {
    const user = userEvent.setup();
    tabs.unsavedPaths = ["notes.txt"];
    renderView();
    await tree();
    await user.click(screen.getByRole("button", { name: "Board" }));
    await user.click(await screen.findByRole("button", { name: "Discard all" }));
    expect(tabs.saveAll).not.toHaveBeenCalled();
    expect(tabs.closeAll).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("Board content")).toBeInTheDocument();
    await waitFor(() => expect(called("files_leave")).toHaveLength(1));
  });

  it("saves every file with Save all and then shows the other view", async () => {
    const user = userEvent.setup();
    tabs.unsavedPaths = ["notes.txt", "src/main.ts"];
    renderView();
    await tree();
    await user.click(screen.getByRole("button", { name: "Board" }));
    await user.click(await screen.findByRole("button", { name: "Save all" }));
    await waitFor(() => expect(tabs.closeAll).toHaveBeenCalledTimes(1));
    expect(tabs.saveAll).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("Board content")).toBeInTheDocument();
  });

  it("stays on the File view when a file cannot be saved: the other view is not shown and the tabs stay", async () => {
    const user = userEvent.setup();
    tabs.unsavedPaths = ["notes.txt", "src/main.ts"];
    tabs.saveAll.mockResolvedValue([{ path: "notes.txt", reason: "The file is read-only." }]);
    renderView();
    await tree();
    await user.click(screen.getByRole("button", { name: "Board" }));
    await user.click(await screen.findByRole("button", { name: "Save all" }));
    await waitFor(() => expect(tabs.saveAll).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
    expect(tabs.closeAll).not.toHaveBeenCalled();
    expect(screen.getByRole("tree", { name: "Project files" })).toBeInTheDocument();
    expect(screen.queryByText("Board content")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Files" })).toHaveAttribute("aria-current", "page");
    expect(called("files_leave")).toEqual([]);
  });
});
