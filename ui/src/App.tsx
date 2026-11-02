import { AppProviders } from "@/components/app-providers";
import { ProjectContent } from "@/components/project-content";
import { WelcomeWindow } from "@/components/welcome-window";
import { OmpProvider } from "@/hooks/use-omp";
import { SettingsProvider, useSettings } from "@/hooks/use-settings";
import { currentWindowLabel } from "@/lib/backend";

const PROJECT_LABEL_PREFIX = "p-";

/** The window's content, themed by the Theme setting (`auto` until the backend answered). */
function Window() {
  const { snapshot } = useSettings();
  const label = currentWindowLabel();
  return (
    <AppProviders preference={snapshot?.settings.theme ?? "auto"}>
      {label.startsWith(PROJECT_LABEL_PREFIX) ? (
        <ProjectContent projectId={label.slice(PROJECT_LABEL_PREFIX.length)} />
      ) : (
        <WelcomeWindow />
      )}
    </AppProviders>
  );
}

/**
 * The window's content by its label: `p-<project hash>` is a project window (sidebar and views),
 * anything else the welcome window. Every window holds the settings the backend sends, so a change
 * of the theme applies to all of them at once.
 */
export default function App() {
  return (
    <SettingsProvider>
      <OmpProvider>
        <Window />
      </OmpProvider>
    </SettingsProvider>
  );
}
