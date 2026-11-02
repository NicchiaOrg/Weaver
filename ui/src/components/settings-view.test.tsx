import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SettingChange } from "@/bindings/SettingChange";
import type { SettingField } from "@/bindings/SettingField";
import type { SettingsSnapshot } from "@/bindings/SettingsSnapshot";
import { SettingsView, type ModelCatalog } from "@/components/settings-view";
import { SettingsProvider } from "@/hooks/use-settings";
import { call, on } from "@/lib/backend";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

const schema: SettingField[] = [
  { key: "ompPath", title: "omp path", description: "Where omp is.", advice: null, kind: "text", defaultValue: "", choices: [] },
  { key: "models.analysis", title: "Analysis", description: "Designs Proposals.", advice: "Your most capable model.", kind: "model", defaultValue: "", choices: [] },
  { key: "models.scan", title: "Scan", description: "Compares the Model.", advice: "A fast, light model.", kind: "model", defaultValue: "", choices: [] },
  { key: "models.execution", title: "Execution", description: "Carries out Tasks.", advice: "A mid-range model.", kind: "model", defaultValue: "", choices: [] },
  { key: "maxParallelAgents", title: "Max parallel sub-agents", description: "How many at once.", advice: null, kind: "integer", defaultValue: "20", choices: [] },
  { key: "language", title: "Language of the agent's texts", description: "Empty means English.", advice: null, kind: "text", defaultValue: "", choices: [] },
  {
    key: "theme",
    title: "Theme",
    description: "Auto follows the system.",
    advice: null,
    kind: "choice",
    defaultValue: "auto",
    choices: [
      { value: "auto", label: "Auto" },
      { value: "light", label: "Light" },
      { value: "dark", label: "Dark" },
    ],
  },
  {
    key: "editor.preset",
    title: "Editor",
    description: "Opens a file at a line.",
    advice: null,
    kind: "choice",
    defaultValue: "vscode",
    choices: [
      { value: "vscode", label: "VS Code" },
      { value: "custom", label: "Custom command" },
    ],
  },
  { key: "editor.command", title: "Custom command", description: "Needs {file}.", advice: null, kind: "text", defaultValue: "", choices: [] },
  {
    key: "permissions",
    title: "Permissions",
    description: "Whether omp asks first.",
    advice: null,
    kind: "choice",
    defaultValue: "yolo",
    choices: [
      { value: "yolo", label: "Run without asking" },
      { value: "ask", label: "Ask before acting" },
    ],
  },
];

function snapshot(revision = 1, patch: Partial<SettingsSnapshot["settings"]> = {}, rest: Partial<SettingsSnapshot> = {}): SettingsSnapshot {
  return {
    revision,
    settings: {
      ompPath: "",
      models: { analysis: "", scan: "", execution: "" },
      maxParallelAgents: 20,
      language: "",
      theme: "auto",
      editor: { preset: "vscode", command: "" },
      permissions: "yolo",
      ...patch,
    },
    issues: [],
    fileError: null,
    schema,
    ...rest,
  };
}

let current: SettingsSnapshot;
let saved: SettingChange[];
let failWrites: string | null;
let push: (next: SettingsSnapshot) => void;

/** A backend that validates the two rules the tests exercise and stores what it accepts. */
function install(initial: SettingsSnapshot) {
  current = initial;
  saved = [];
  failWrites = null;
  vi.mocked(on).mockImplementation(((_name: string, handler: (next: SettingsSnapshot) => void) => {
    push = handler;
    return Promise.resolve(() => {});
  }) as unknown as typeof on);
  vi.mocked(call).mockImplementation(((name: string, args: { change?: SettingChange }) => {
    if (name === "settings_get") return Promise.resolve(current);
    const change = args.change!;
    saved.push(change);
    if (change.key === "maxParallelAgents" && !/^[1-9][0-9]*$/.test(change.value)) {
      return Promise.reject({ code: "invalid-input", message: "Enter a whole number of at least 1." });
    }
    if (change.key === "editor.command" && !change.value.includes("{file}")) {
      return Promise.reject({ code: "invalid-input", message: "The command must contain {file}." });
    }
    if (failWrites) return Promise.reject({ code: "internal", message: failWrites });
    const s = structuredClone(current.settings);
    switch (change.key) {
      case "maxParallelAgents":
        s.maxParallelAgents = Number(change.value);
        break;
      case "language":
        s.language = change.value;
        break;
      case "theme":
        s.theme = change.value as typeof s.theme;
        break;
      case "permissions":
        s.permissions = change.value as typeof s.permissions;
        break;
      case "editor.preset":
        s.editor.preset = change.value as typeof s.editor.preset;
        break;
      case "editor.command":
        s.editor.command = change.value;
        break;
      case "models.analysis":
        s.models.analysis = change.value;
        break;
      default:
        break;
    }
    current = { ...current, revision: current.revision + 1, settings: s };
    return Promise.resolve(current);
  }) as unknown as typeof call);
}

async function show(props: Parameters<typeof SettingsView>[0] = {}) {
  render(
    <SettingsProvider>
      <SettingsView {...props} />
    </SettingsProvider>,
  );
  await screen.findByRole("heading", { name: "omp" });
}

beforeEach(() => install(snapshot()));

describe("settings view", () => {
  it("shows the groups in order and never a setting key", async () => {
    await show({ projectSlot: <p>Update skills box</p> });
    const headings = screen.getAllByRole("heading", { level: 2 }).map((h) => h.textContent);
    expect(headings).toEqual(["omp", "Models", "Agents", "Editor", "Appearance", "Project"]);
    expect(screen.getByText("Update skills box")).toBeInTheDocument();
    for (const key of ["ompPath", "maxParallelAgents", "models.analysis", "editor.preset"]) {
      expect(screen.queryByText(key)).not.toBeInTheDocument();
    }
  });

  it("shows the Project group only with its slot, and the omp slot first in its group", async () => {
    await show({ ompSlot: <p>omp status box</p> });
    expect(screen.queryByRole("heading", { name: "Project" })).not.toBeInTheDocument();
    const group = screen.getByRole("region", { name: "omp" });
    expect(group.firstElementChild?.nextElementSibling?.textContent).toBe("omp status box");
  });

  it("shows titles, bold advice and descriptions from the schema", async () => {
    await show();
    expect(screen.getByLabelText("Analysis")).toBeInTheDocument();
    const advice = screen.getByText("Your most capable model.");
    expect(advice.tagName).toBe("STRONG");
    expect(screen.getByText("Empty means English.")).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Run without asking" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "Auto" })).toBeChecked();
  });

  it("does not save an invalid value: the field keeps the text and says why", async () => {
    const user = userEvent.setup();
    await show();
    const field = screen.getByLabelText("Max parallel sub-agents");
    await user.clear(field);
    await user.type(field, "0");
    await user.tab();
    expect(await screen.findByText("Enter a whole number of at least 1.")).toBeInTheDocument();
    expect(field).toHaveValue("0");
    expect(field).toHaveAttribute("aria-invalid", "true");
    expect(current.settings.maxParallelAgents).toBe(20);
  });

  it("saves a number on Enter and clears the refusal", async () => {
    const user = userEvent.setup();
    await show();
    const field = screen.getByLabelText("Max parallel sub-agents");
    await user.clear(field);
    await user.type(field, "x{Enter}");
    expect(await screen.findByText("Enter a whole number of at least 1.")).toBeInTheDocument();
    await user.clear(field);
    await user.type(field, "8{Enter}");
    await vi.waitFor(() => expect(current.settings.maxParallelAgents).toBe(8));
    expect(screen.queryByText("Enter a whole number of at least 1.")).not.toBeInTheDocument();
    expect(field).toHaveValue("8");
  });

  it("saves text on blur and sends nothing when the value did not change", async () => {
    const user = userEvent.setup();
    await show();
    const field = screen.getByLabelText("Language of the agent's texts");
    await user.click(field);
    await user.tab();
    expect(saved).toEqual([]);
    await user.type(field, "  Italian ");
    await user.tab();
    await vi.waitFor(() => expect(saved).toEqual([{ key: "language", value: "Italian" }]));
  });

  it("shows a failed write under the field and puts the stored value back", async () => {
    const user = userEvent.setup();
    await show();
    failWrites = "cannot write settings.json (disk full)";
    const field = screen.getByLabelText("Language of the agent's texts");
    await user.type(field, "French");
    await user.tab();
    expect(
      await screen.findByText("Could not save Language of the agent's texts: cannot write settings.json (disk full)."),
    ).toBeInTheDocument();
    expect(field).toHaveValue("");
    expect(current.settings.language).toBe("");
  });

  it("saves a choice at once, with no confirmation", async () => {
    const user = userEvent.setup();
    await show();
    await user.click(screen.getByRole("radio", { name: "Dark" }));
    await vi.waitFor(() => expect(saved).toEqual([{ key: "theme", value: "dark" }]));
    expect(await screen.findByRole("radio", { name: "Dark" })).toBeChecked();
    await user.click(screen.getByRole("radio", { name: "Ask before acting" }));
    await vi.waitFor(() => expect(saved.at(-1)).toEqual({ key: "permissions", value: "ask" }));
  });

  it("shows a change from another window at once, except in the field with the focus", async () => {
    const user = userEvent.setup();
    await show();
    const language = screen.getByLabelText("Language of the agent's texts");
    await user.click(language);
    act(() => push(snapshot(5, { theme: "dark", language: "Italian", ompPath: "/opt/omp" })));
    expect(screen.getByRole("radio", { name: "Dark" })).toBeChecked();
    expect(screen.getByLabelText("omp path")).toHaveValue("/opt/omp");
    expect(language).toHaveValue("");
    await user.tab();
    expect(language).toHaveValue("Italian");
    expect(saved).toEqual([]);
  });

  it("keeps what the developer typed in a focused field when another window changes it", async () => {
    const user = userEvent.setup();
    await show();
    const language = screen.getByLabelText("Language of the agent's texts");
    await user.type(language, "Fr");
    act(() => push(snapshot(5, { language: "Italian" })));
    expect(language).toHaveValue("Fr");
  });

  it("ignores a snapshot older than the one it holds", async () => {
    await show();
    act(() => push(snapshot(7, { theme: "dark" })));
    act(() => push(snapshot(3, { theme: "light" })));
    expect(screen.getByRole("radio", { name: "Dark" })).toBeChecked();
  });

  it("shows the custom command only with the Custom command preset, and refuses one without {file}", async () => {
    const user = userEvent.setup();
    install(snapshot(1, { editor: { preset: "custom", command: "" } }));
    await show();
    const field = screen.getByLabelText("Custom command");
    await user.type(field, "myeditor");
    await user.tab();
    expect(await screen.findByText("The command must contain {file}.")).toBeInTheDocument();
    expect(field).toHaveValue("myeditor");
    await user.type(field, " {{file}:{{line}");
    await user.tab();
    await vi.waitFor(() => expect(current.settings.editor.command).toBe("myeditor {file}:{line}"));
  });

  it("hides the custom command with a preset", async () => {
    await show();
    expect(screen.queryByLabelText("Custom command")).not.toBeInTheDocument();
  });

  it("tells which key of the file was ignored and why, and shows an unreadable file", async () => {
    install(
      snapshot(1, {}, {
        issues: [{ key: "maxParallelAgents", reason: "Enter a whole number of at least 1." }],
        fileError: "expected value at line 1 column 1",
      }),
    );
    await show();
    expect(screen.getByText(/The value in the settings file was ignored.*Enter a whole number of at least 1\./)).toBeInTheDocument();
    expect(screen.getByText(/The settings file could not be used: expected value at line 1 column 1\. Every default is in force/)).toBeInTheDocument();
  });

  it("shows a failed load with Retry", async () => {
    const user = userEvent.setup();
    vi.mocked(call).mockRejectedValueOnce({ code: "internal", message: "disk gone" });
    vi.mocked(on).mockImplementation((() => Promise.resolve(() => {})) as unknown as typeof on);
    render(
      <SettingsProvider>
        <SettingsView />
      </SettingsProvider>,
    );
    expect(await screen.findByText("The settings could not be loaded: disk gone")).toBeInTheDocument();
    vi.mocked(call).mockResolvedValueOnce(snapshot());
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByRole("heading", { name: "omp" })).toBeInTheDocument();
  });
});

describe("model roles", () => {
  const listed: ModelCatalog = {
    status: "listed",
    models: [
      { selector: "anthropic/opus", name: "Claude Opus", thinkingLevels: ["low", "high"] },
      { selector: "openai/mini", name: "openai/mini" },
    ],
  };

  it("offers `omp default` and the listed models, filtered while typing, and saves a choice at once", async () => {
    const user = userEvent.setup();
    await show({ modelCatalog: listed });
    const field = screen.getByLabelText("Analysis");
    await user.click(field);
    const options = within(await screen.findByRole("listbox")).getAllByRole("option");
    expect(options.map((o) => o.textContent)).toEqual(["omp default", "anthropic/opusClaude Opus", "openai/mini"]);
    await user.type(field, "opus");
    expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(1);
    await user.click(screen.getByRole("option", { name: /anthropic\/opus/ }));
    await vi.waitFor(() => expect(saved).toEqual([{ key: "models.analysis", value: "anthropic/opus" }]));
  });

  it("keeps free text that no list contains, saved on Enter", async () => {
    const user = userEvent.setup();
    await show({ modelCatalog: listed });
    await user.type(screen.getByLabelText("Analysis"), "my/custom-model{Enter}");
    await vi.waitFor(() => expect(saved).toEqual([{ key: "models.analysis", value: "my/custom-model" }]));
  });

  it("picks a suggestion with the arrows and Enter, and closes the list with Esc", async () => {
    const user = userEvent.setup();
    await show({ modelCatalog: listed });
    const field = screen.getByLabelText("Analysis");
    await user.click(field);
    await user.keyboard("{ArrowDown}{ArrowDown}{Enter}");
    await vi.waitFor(() => expect(saved).toEqual([{ key: "models.analysis", value: "anthropic/opus" }]));
    await user.keyboard("{ArrowDown}");
    expect(await screen.findByRole("listbox")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("shows the thinking level Select only for a model with listed levels, and shows the model only in the field", async () => {
    install(snapshot(1, { models: { analysis: "anthropic/opus:high", scan: "openai/mini", execution: "" } }));
    await show({ modelCatalog: listed });
    expect(screen.getByLabelText("Analysis")).toHaveValue("anthropic/opus");
    const level = screen.getByRole("combobox", { name: "Analysis thinking level" });
    expect(level).toHaveTextContent("high");
    expect(screen.queryByRole("combobox", { name: "Scan thinking level" })).not.toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "Execution thinking level" })).not.toBeInTheDocument();
  });

  it("treats a suffix omp does not list as part of the model name", async () => {
    install(snapshot(1, { models: { analysis: "anthropic/opus:turbo", scan: "", execution: "" } }));
    await show({ modelCatalog: listed });
    expect(screen.getByLabelText("Analysis")).toHaveValue("anthropic/opus:turbo");
    expect(screen.queryByRole("combobox", { name: "Analysis thinking level" })).not.toBeInTheDocument();
  });

  it("says why the list is unavailable, and still takes typing", async () => {
    const user = userEvent.setup();
    await show({ modelCatalog: { status: "unavailable", reason: "omp not found at /opt/omp" } });
    expect(screen.getByText("omp not found at /opt/omp")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Scan"), "x/y");
    await user.tab();
    await vi.waitFor(() => expect(saved).toEqual([{ key: "models.scan", value: "x/y" }]));
  });
});
