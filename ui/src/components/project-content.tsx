import { BoardProvider, useBoard } from "@/components/board/board-state";
import { BoardView } from "@/components/board/board-view";
import { GeneralView } from "@/components/general/general-view";
import { HistoryView } from "@/components/history/history-view";
import { JobsBoardActions } from "@/components/jobs/board-actions";
import { JobsView } from "@/components/jobs/jobs-view";
import { JobsProvider, useJobs } from "@/components/jobs/use-jobs";
import { ConnectedSettingsView } from "@/components/omp-status";
import { ProjectWindow } from "@/components/project-window";
import { QuestionRoundLayer } from "@/components/questions/question-overlay";
import { RequestsBoardActions, RequestsLayer } from "@/components/requests/requests-layer";
import { SetupLayer } from "@/components/project/empty-model";
import { ProjectGate } from "@/components/project/project-gate";
import { OpeningCheckToast, SkillsBanner, UpdateSkills } from "@/components/project/update-skills";
import { ProjectStatusProvider } from "@/components/project/use-project-status";
import { RequestsProvider } from "@/components/requests/use-requests";
import { RequirementsView } from "@/components/views/requirements-view";
import { SpecsView } from "@/components/views/specs-view";
import { ViewsProvider } from "@/components/views/views-state";

function ProjectShell({ projectId }: { projectId: string }) {
  const { snapshot } = useBoard();
  const { waiting } = useJobs();
  return (
    <ProjectWindow
      projectId={projectId}
      views={{
        board: (
          <ProjectGate>
            <RequestsLayer projectId={projectId}>
              <SetupLayer>
                <BoardView />
              </SetupLayer>
            </RequestsLayer>
          </ProjectGate>
        ),
        requirements: (
          <ProjectGate>
            <RequirementsView />
          </ProjectGate>
        ),
        specs: (
          <ProjectGate>
            <SpecsView />
          </ProjectGate>
        ),
        history: (
          <ProjectGate>
            <HistoryView />
          </ProjectGate>
        ),
        jobs: (
          <ProjectGate>
            <JobsView />
          </ProjectGate>
        ),
        general: (
          <ProjectGate>
            <GeneralView />
          </ProjectGate>
        ),
        // Settings is the one view that works without a project.
        settings: <ConnectedSettingsView projectSlot={<UpdateSkills />} />,
      }}
      // What the check of the agent files says when the window opens: a toast, or a banner with Retry.
      banners={
        <>
          <OpeningCheckToast />
          <SkillsBanner />
        </>
      }
      // The sidebar shows the real cards of Backlog (ghosts never count) and the jobs waiting for
      // the developer; the backend counts them.
      counts={{ backlog: snapshot?.backlog.length ?? 0, jobsWaiting: waiting }}
      // What Requirements and Specs share, and the cards' `View requirements`; `Run`, `Run all` and
      // `Scan System` start jobs.
      around={(children) => (
        <ViewsProvider projectId={projectId}>
          <JobsBoardActions>
            <RequestsBoardActions>{children}</RequestsBoardActions>
          </JobsBoardActions>
        </ViewsProvider>
      )}
    />
  );
}

/** What a project window shows: the views of the project, fed by what the backend holds for it. */
export function ProjectContent({ projectId }: { projectId: string }) {
  return (
    <ProjectStatusProvider projectId={projectId}>
      <BoardProvider projectId={projectId}>
        <JobsProvider projectId={projectId}>
          <RequestsProvider projectId={projectId}>
            {/* The round the agent (or Weaver, for a Drift) asks: a modal overlay over every view. */}
            <QuestionRoundLayer projectId={projectId}>
              <ProjectShell projectId={projectId} />
            </QuestionRoundLayer>
          </RequestsProvider>
        </JobsProvider>
      </BoardProvider>
    </ProjectStatusProvider>
  );
}
