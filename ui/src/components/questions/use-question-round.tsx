import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { toast } from "sonner";

import type { ActionInput } from "@/bindings/ActionInput";
import type { QuestionRoundView } from "@/bindings/QuestionRoundView";
import { call, isAppError, on } from "@/lib/backend";

/** The round presented in this project window, and what the developer does in it. */
export interface QuestionRoundState {
  /** The round on screen; `null` when none is presented. The backend keeps it, drafts included. */
  round: QuestionRoundView | null;
  /** Sends what the developer did, in order; the answer of the backend replaces the round on screen. */
  act: (action: ActionInput) => void;
}

const QuestionRoundContext = createContext<QuestionRoundState | null>(null);

export function useQuestionRound(): QuestionRoundState {
  const value = useContext(QuestionRoundContext);
  if (!value) throw new Error("useQuestionRound needs a QuestionRoundProvider");
  return value;
}

/** The round of the window when it has the layer (the Jobs view also renders without it). */
export function useOptionalQuestionRound(): QuestionRoundState | null {
  return useContext(QuestionRoundContext);
}

/** What is on screen once a draft was written, before the backend answers. */
function withDraft(round: QuestionRoundView, action: Extract<ActionInput, { kind: "draft" }>): QuestionRoundView {
  return {
    ...round,
    questions: round.questions.map((question, index) =>
      index === action.index ? { ...question, draft: { selected: action.selected, text: action.text } } : question,
    ),
  };
}

export function QuestionRoundProvider({ projectId, children }: { projectId: string; children: ReactNode }) {
  const [round, setRound] = useState<QuestionRoundView | null>(null);
  const latest = useRef<QuestionRoundView | null>(null);
  const alive = useRef(true);
  const heard = useRef(false);
  /** The actions go to the backend one after the other: a command may not overtake the one before it. */
  const chain = useRef<Promise<void>>(Promise.resolve());
  const issued = useRef(0);

  const show = useCallback((next: QuestionRoundView | null) => {
    latest.current = next;
    setRound(next);
  }, []);

  useEffect(() => {
    alive.current = true;
    heard.current = false;
    // Listen first, then ask: a change between the two is not lost, and an answer older than an event is dropped.
    const changed = on("question-round-changed", (event) => {
      if (!alive.current || event.projectId !== projectId) return;
      heard.current = true;
      show(event.round);
    });
    call("question_round_get", { projectId })
      .then((current) => {
        if (alive.current && !heard.current) show(current);
      })
      .catch(() => {});
    return () => {
      alive.current = false;
      changed.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [projectId, show]);

  const act = useCallback(
    (action: ActionInput) => {
      const current = latest.current;
      if (!current) return;
      const id = current.id;
      const ticket = ++issued.current;
      if (action.kind === "draft") show(withDraft(current, action));
      chain.current = chain.current.then(async () => {
        try {
          const next = await call("question_round_act", { projectId, roundId: id, action });
          // A newer action is on its way: its answer is the one that counts.
          if (!alive.current || ticket !== issued.current) return;
          if (latest.current === null || latest.current.id === id) show(next);
        } catch (error) {
          if (alive.current) toast.error(isAppError(error) ? error.message : "The answer could not be recorded.");
        }
      });
    },
    [projectId, show],
  );

  const value = useMemo(() => ({ round, act }), [round, act]);
  return <QuestionRoundContext.Provider value={value}>{children}</QuestionRoundContext.Provider>;
}
