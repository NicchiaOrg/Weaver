# Weaver

Weaver is a desktop app for programming from a visual and textual Model of every state a software system can be in, plus the Requirements that say how it must be built. The developer reviews and decides on changes to the Model; a coding agent, omp, explores the Model, proposes changes as Proposals, compares the Model with the code and files the differences as Drifts, and carries out the work as Tasks. Weaver never calls a language model and never reads the developer's code: the agent does.

The Model, the Requirements, the Specs and the cards (Proposals, Tasks, Drifts, History entries) are files of the developer's project, kept in its repository with no database. Nothing changes in the Model until the developer approves a Proposal as a whole; every other decision on the work is the developer's too, and the agent takes one only when the developer explicitly asks for it.

Weaver is used through two surfaces that share one behaviour: the app, where the developer looks at the project and decides, and the `weaver` command line, which is how the agent reaches Weaver from inside the conversations Weaver starts. One app process serves every open project, each in its own window, and Weaver stays next to the developer's own editor, opening files there when asked.
