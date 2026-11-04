import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

import { NotesSession, type NotesEditorModule } from "@/components/notes/notes-session";

const NotesContext = createContext<NotesSession | null>(null);

/** The state of the Notes view of this project window. */
export function useNotes(): NotesSession {
  const session = useContext(NotesContext);
  if (!session) throw new Error("useNotes needs a NotesProvider");
  return session;
}

/**
 * Holds the Notes view's state for the life of a project window (the editor with its history, the
 * hash of the sheet shown, the removed images and the image cache: UI memory only), and answers the
 * backend's request for the waiting text before the window closes, whether or not the view was
 * ever opened. Nothing is read from disk until the view is shown.
 */
export function NotesProvider({ projectId, children, loadModule }: { projectId: string; children: ReactNode; loadModule?: () => Promise<NotesEditorModule> }) {
  const [session] = useState(() => new NotesSession(projectId, loadModule));
  useEffect(() => {
    session.start();
    return () => session.dispose();
  }, [session]);
  return <NotesContext.Provider value={session}>{children}</NotesContext.Provider>;
}
