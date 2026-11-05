# Model changes

How the Model, the Requirements and the Specs change: the agent writes a change as a Proposal, and nothing takes effect until the developer approves it as a whole. A Proposal is the developer's to decide only once the agent has finished writing it and handed it over; until then it is a Draft. A Proposal can be written on top of other Proposals it requires, and can then be approved only after them. An ordinary change carries the Tasks that carry it out, and approving it creates them at once, with no scan.

Every Proposal, Task and Drift has a permanent identity of its own, drawn at random so that two branches never give the same one, used in every file, field and instruction; next to it a live card has a short number (`P3`, `T3`, `D3`), local to each working copy and freed when the card closes, only for reading and citing it at a glance.

A design starts from a written request: the developer writes, inside Weaver, a long and well structured Markdown document, and sends it when it is ready. The agent interviews him with question rounds and opens the Proposals; the first one is bound to the request, and handing it over consumes the request.
