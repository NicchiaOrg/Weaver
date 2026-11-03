import { ChevronDown, ChevronRight, File, FilePlus, Folder, FolderInput, FolderOpen, FolderPlus, Lock, Pencil, Trash2 } from "lucide-react";
import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type DragEvent,
  type KeyboardEvent,
  type MouseEvent,
  type ReactNode,
} from "react";

import type { FileEntry } from "@/bindings/FileEntry";
import { useFileTabs } from "@/components/files/file-tabs-context";
import {
  AGENT_EDITING,
  canDrop,
  inWeaver,
  nameOf,
  NEW_ROW,
  ROOT,
  selection,
  WEAVER_READ_ONLY,
  type Row,
} from "@/components/files/tree-model";
import type { FileTree as Tree } from "@/components/files/use-file-tree";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuSeparator, ContextMenuTrigger } from "@/components/ui/context-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

/** What the view can ask of the tree from outside. */
export interface FileTreeHandle {
  /** Puts the focus on the selected row (or the first). */
  focus: () => void;
}

export interface FileTreeProps {
  tree: Tree;
  /** `Move to…` was chosen for this path. */
  onRequestMove: (path: string) => void;
  /** `Delete` (menu or key) was chosen for this path. */
  onRequestDelete: (path: string) => void;
}

const REASON_ID = "file-tree-read-only-reason";

/** Focus the row that is selected when the edit of a name ends. */
const SELECTED = Symbol("selected");

/** The text field of a row being made or renamed: Enter confirms, Esc or leaving the field cancels. */
function NameField({
  initial,
  folder,
  label,
  onCommit,
  onCancel,
}: {
  initial: string;
  folder: boolean;
  label: string;
  onCommit: (name: string) => Promise<boolean>;
  /** The field ended without a name: `refocus` is whether the row gets the focus (Esc), not when the developer went elsewhere. */
  onCancel: (refocus: boolean) => void;
}) {
  const field = useRef<HTMLInputElement>(null);
  const finished = useRef(false);

  useEffect(() => {
    const input = field.current;
    if (!input) return;
    input.focus();
    const [from, to] = selection(initial, folder);
    input.setSelectionRange(from, to);
  }, [initial, folder]);

  const confirm = () => {
    if (finished.current) return;
    finished.current = true;
    onCommit(field.current?.value ?? "")
      .then((done) => {
        // A refused name stays in the field, for the developer to correct.
        if (!done) finished.current = false;
      })
      .catch(() => {
        finished.current = false;
      });
  };

  return (
    <input
      ref={field}
      type="text"
      aria-label={label}
      defaultValue={initial}
      spellCheck={false}
      autoComplete="off"
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Enter") {
          event.preventDefault();
          confirm();
        } else if (event.key === "Escape") {
          event.preventDefault();
          finished.current = true;
          onCancel(true);
        }
      }}
      onBlur={() => {
        if (finished.current) return;
        finished.current = true;
        onCancel(false);
      }}
      className="h-6 min-w-0 flex-1 rounded-sm border bg-background px-1 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    />
  );
}

/** A menu item that is shown disabled with the reason, in a tooltip and as its description. */
function BlockedItem({ children, reason }: { children: ReactNode; reason: string }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <ContextMenuItem aria-disabled="true" aria-describedby={REASON_ID} onSelect={(event) => event.preventDefault()}>
          {children}
        </ContextMenuItem>
      </TooltipTrigger>
      <TooltipContent side="right">{reason}</TooltipContent>
    </Tooltip>
  );
}

const isMac = () => typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);

/**
 * The tree of the project's files: an ARIA `tree` with the entries the backend lists. Up and Down
 * move, Right opens a folder or goes to its first child, Left closes it or goes to its parent,
 * Enter opens a file in a tab or toggles a folder (a click does the same), `F2` renames and `Delete`
 * (`Cmd+Backspace` on macOS) deletes the selected row. A right click, the Menu key or `Shift+F10`
 * opens the menu of the row, and of the root on the empty space. A row dragged onto a folder, or
 * onto the empty space, moves there.
 */
export const FileTree = forwardRef<FileTreeHandle, FileTreeProps>(function FileTree({ tree, onRequestMove, onRequestDelete }, handle) {
  const tabs = useFileTabs();
  const container = useRef<HTMLDivElement>(null);
  const items = useRef(new Map<string, HTMLElement>());
  /** The row to focus once it is on screen: a path, or `SELECTED` for the row selected by then. */
  const wantFocus = useRef<string | typeof SELECTED | null>(null);
  const keepFocus = useRef(false);
  const [menuTarget, setMenuTarget] = useState<string>(ROOT);
  const [dragging, setDragging] = useState<FileEntry | null>(null);
  const [dropTarget, setDropTarget] = useState<string | null>(null);

  const { rows, selected, editing } = tree;
  const current = rows.find((row) => !row.isNew && row.entry.path === selected) ?? rows.find((row) => !row.isNew) ?? null;

  // Moves the focus once the row the arrows went to is on screen.
  useEffect(() => {
    const wanted = wantFocus.current;
    if (wanted === null) return;
    const path = wanted === SELECTED ? selected : wanted;
    const node = path === null ? undefined : items.current.get(path);
    if (!node) return;
    wantFocus.current = null;
    node.focus();
  });

  useImperativeHandle(handle, () => ({
    focus: () => {
      const path = current?.entry.path;
      if (path !== undefined) items.current.get(path)?.focus();
    },
  }));

  const moveTo = (path: string) => {
    wantFocus.current = path;
    tree.select(path);
  };

  const openEntry = (entry: FileEntry) => {
    if (entry.kind === "folder") tree.toggle(entry.path);
    else tabs.openFile(entry.path);
  };

  const requestRename = (path: string) => {
    if (!inWeaver(path)) tree.startRename(path);
  };

  const requestDelete = (path: string) => {
    if (!inWeaver(path)) onRequestDelete(path);
  };

  const openMenuOn = (row: HTMLElement | null) => {
    const target = row ?? container.current;
    if (!target) return;
    const rect = target.getBoundingClientRect();
    target.dispatchEvent(
      new MouseEvent("contextmenu", { bubbles: true, cancelable: true, button: 2, clientX: rect.left + 16, clientY: rect.top + rect.height / 2 }),
    );
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target instanceof HTMLInputElement) return;
    if (!current) return;
    const { entry } = current;
    const real = rows.filter((row) => !row.isNew);
    const position = real.indexOf(current);
    const next = real[position + 1];
    const previous = real[position - 1];
    if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
      event.preventDefault();
      openMenuOn(items.current.get(entry.path) ?? null);
      return;
    }
    if (event.altKey || event.ctrlKey || (event.metaKey && event.key !== "Backspace")) return;
    switch (event.key) {
      case "ArrowDown":
        if (next) moveTo(next.entry.path);
        break;
      case "ArrowUp":
        if (previous) moveTo(previous.entry.path);
        break;
      case "Home":
        moveTo(real[0]?.entry.path ?? entry.path);
        break;
      case "End":
        moveTo(real.at(-1)?.entry.path ?? entry.path);
        break;
      case "ArrowRight":
        if (entry.kind !== "folder") break;
        if (!tree.isExpanded(entry.path)) tree.expand(entry.path);
        else if (next?.parent === entry.path) moveTo(next.entry.path);
        break;
      case "ArrowLeft":
        if (entry.kind === "folder" && tree.isExpanded(entry.path)) tree.collapse(entry.path);
        else if (current.parent !== ROOT) moveTo(current.parent);
        break;
      case "Enter":
        openEntry(entry);
        break;
      case "F2":
        requestRename(entry.path);
        break;
      case "Delete":
        requestDelete(entry.path);
        break;
      case "Backspace":
        if (!(event.metaKey && isMac())) return;
        requestDelete(entry.path);
        break;
      default:
        return;
    }
    event.preventDefault();
  };

  // --- the menu -------------------------------------------------------------------------------

  const rowOf = (event: { target: EventTarget }) => (event.target as HTMLElement).closest<HTMLElement>('[role="treeitem"]');

  const onContextMenu = (event: MouseEvent<HTMLDivElement>) => {
    const row = rowOf(event);
    const path = row?.dataset.path;
    if (path === undefined || path === NEW_ROW) {
      setMenuTarget(ROOT);
      return;
    }
    setMenuTarget(path);
    tree.select(path);
  };

  const target = menuTarget === ROOT ? null : tree.entryOf(menuTarget);
  const blocked = inWeaver(menuTarget);
  const holdsFolder = menuTarget === ROOT || target?.kind === "folder";

  const choose = (action: () => void) => () => {
    keepFocus.current = true;
    action();
  };

  const item = (label: string, icon: ReactNode, action: () => void) =>
    blocked ? (
      <BlockedItem reason={WEAVER_READ_ONLY}>
        {icon}
        {label}
      </BlockedItem>
    ) : (
      <ContextMenuItem onSelect={choose(action)}>
        {icon}
        {label}
      </ContextMenuItem>
    );

  // --- dragging -------------------------------------------------------------------------------

  /** The folder a drop on this event's target would go to: a folder row, or the empty space for the root. */
  const dropFolderOf = (event: DragEvent<HTMLDivElement>): string | null => {
    if (!dragging) return null;
    const row = rowOf(event);
    const path = row?.dataset.path;
    let folder: string | null;
    if (path === undefined) folder = ROOT;
    else folder = tree.entryOf(path)?.kind === "folder" ? path : null;
    return folder !== null && canDrop(dragging, folder) ? folder : null;
  };

  const endDrag = () => {
    setDragging(null);
    setDropTarget(null);
  };

  const onDragOver = (event: DragEvent<HTMLDivElement>) => {
    const folder = dropFolderOf(event);
    setDropTarget(folder);
    if (folder === null) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
  };

  const onDrop = (event: DragEvent<HTMLDivElement>) => {
    const folder = dropFolderOf(event);
    const source = dragging;
    endDrag();
    if (folder === null || !source) return;
    event.preventDefault();
    tree.move(source.path, folder).catch(() => {});
  };

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div
          ref={container}
          role="tree"
          aria-label="Project files"
          aria-multiselectable="false"
          tabIndex={current ? undefined : 0}
          onKeyDown={onKeyDown}
          onContextMenu={onContextMenu}
          onDragOver={onDragOver}
          onDrop={onDrop}
          onDragLeave={(event) => {
            if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDropTarget(null);
          }}
          data-drop-target={dropTarget === ROOT ? "" : undefined}
          className={cn("min-h-full py-3 text-sm", dropTarget === ROOT && "bg-accent/50")}
        >
          {rows.map((row) => (
            <TreeRow
              key={row.entry.path}
              row={row}
              tree={tree}
              focusable={current?.entry.path === row.entry.path}
              locked={tabs.lockedPaths.has(row.entry.path)}
              dropTarget={dropTarget === row.entry.path}
              register={(node) => {
                if (node) items.current.set(row.entry.path, node);
                else items.current.delete(row.entry.path);
              }}
              onOpen={() => openEntry(row.entry)}
              onEditEnd={(refocus) => {
                tree.stopEditing();
                if (refocus) wantFocus.current = SELECTED;
              }}
              onDragStart={() => setDragging(row.entry)}
              onDragEnd={endDrag}
            />
          ))}
          {editing === null && rows.length === 0 && tree.ready ? <p className="px-3 py-2 text-muted-foreground">This folder is empty.</p> : null}
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent
        aria-label="File actions"
        onCloseAutoFocus={(event) => {
          // A name being edited takes the focus; a dialog keeps its own; otherwise the row gets it back.
          event.preventDefault();
          const field = container.current?.querySelector("input");
          if (field) field.focus();
          else if (!keepFocus.current) (current ? items.current.get(selected ?? current.entry.path) : container.current)?.focus();
          keepFocus.current = false;
        }}
      >
        <span id={REASON_ID} className="sr-only">
          {WEAVER_READ_ONLY}
        </span>
        {holdsFolder ? item("New file", <FilePlus aria-hidden />, () => tree.startCreate(menuTarget, false)) : null}
        {holdsFolder ? item("New folder", <FolderPlus aria-hidden />, () => tree.startCreate(menuTarget, true)) : null}
        {menuTarget !== ROOT ? (
          <>
            <ContextMenuSeparator />
            {item("Rename", <Pencil aria-hidden />, () => tree.startRename(menuTarget))}
            {item("Move to…", <FolderInput aria-hidden />, () => onRequestMove(menuTarget))}
            {item("Delete", <Trash2 aria-hidden />, () => onRequestDelete(menuTarget))}
          </>
        ) : null}
      </ContextMenuContent>
    </ContextMenu>
  );
});

function TreeRow({
  row,
  tree,
  focusable,
  locked,
  dropTarget,
  register,
  onOpen,
  onEditEnd,
  onDragStart,
  onDragEnd,
}: {
  row: Row;
  tree: Tree;
  focusable: boolean;
  locked: boolean;
  dropTarget: boolean;
  register: (node: HTMLElement | null) => void;
  onOpen: () => void;
  /** The edit of the name ended; `refocus` says the row should have the focus. */
  onEditEnd: (refocus: boolean) => void;
  onDragStart: () => void;
  onDragEnd: () => void;
}) {
  const { entry } = row;
  const folder = entry.kind === "folder";
  const expanded = folder && tree.isExpanded(entry.path);
  const Icon = folder ? (expanded ? FolderOpen : Folder) : File;
  const renaming = tree.editing?.mode === "rename" && tree.editing.path === entry.path;
  const creating = row.isNew === true;
  const readOnly = inWeaver(entry.path);

  let label: ReactNode = <span className="min-w-0 flex-1 truncate">{entry.name}</span>;
  if (creating && tree.editing?.mode === "create") {
    const { parent, folder: isFolder } = tree.editing;
    label = (
      <NameField
        initial=""
        folder={isFolder}
        label={isFolder ? "New folder name" : "New file name"}
        onCommit={async (name) => {
          const done = await tree.create(parent, name, isFolder);
          if (done) onEditEnd(true);
          return done;
        }}
        onCancel={onEditEnd}
      />
    );
  } else if (renaming) {
    label = (
      <NameField
        initial={entry.name}
        folder={folder}
        label={`New name for ${nameOf(entry.path)}`}
        onCommit={async (name) => {
          const done = name === entry.name || (await tree.rename(entry.path, name));
          if (done) onEditEnd(true);
          return done;
        }}
        onCancel={onEditEnd}
      />
    );
  }

  return (
    <div
      role="treeitem"
      ref={register}
      data-path={entry.path}
      data-drop-target={dropTarget ? "" : undefined}
      tabIndex={focusable ? 0 : -1}
      aria-level={row.level}
      aria-setsize={row.setSize}
      aria-posinset={row.position}
      aria-expanded={folder && !creating ? expanded : undefined}
      aria-selected={tree.selected === entry.path}
      draggable={!creating && !renaming && !readOnly}
      onFocus={(event) => {
        if (event.target === event.currentTarget && !creating) tree.select(entry.path);
      }}
      onClick={() => {
        if (creating || renaming) return;
        tree.select(entry.path);
        onOpen();
      }}
      onDragStart={(event) => {
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData("text/plain", entry.path);
        onDragStart();
      }}
      onDragEnd={onDragEnd}
      style={{ paddingLeft: `${(row.level - 1) * 16 + 8}px` }}
      className={cn(
        "flex cursor-pointer items-center gap-2 py-1 pr-3 hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring",
        tree.selected === entry.path && "bg-accent/60",
        dropTarget && "bg-accent ring-2 ring-inset ring-ring",
      )}
    >
      {folder ? (
        expanded ? (
          <ChevronDown aria-hidden className="size-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight aria-hidden className="size-4 shrink-0 text-muted-foreground" />
        )
      ) : (
        <span aria-hidden className="size-4 shrink-0" />
      )}
      <Icon aria-hidden className="size-4 shrink-0 text-muted-foreground" />
      {label}
      {locked ? (
        <Tooltip>
          <TooltipTrigger asChild>
            <span role="img" aria-label={AGENT_EDITING} className="shrink-0 text-muted-foreground">
              <Lock aria-hidden className="size-3.5" />
            </span>
          </TooltipTrigger>
          <TooltipContent>{AGENT_EDITING}</TooltipContent>
        </Tooltip>
      ) : null}
    </div>
  );
}
