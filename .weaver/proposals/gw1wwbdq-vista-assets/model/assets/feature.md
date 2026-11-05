# Assets

The files the developer keeps in the project for the agent, such as images, documents or examples, in one assets folder inside `.weaver`, versioned with the project like the Model. The folder is flat: it holds files only, no folders. It is created when the first asset arrives.

The Assets view of a project window shows the assets as a grid of icons, each chosen by the kind of file, like a simple Finder or Explorer window. From it the developer adds files by dragging them in from outside Weaver, which copies them and leaves the originals where they are; renames an asset; deletes assets, which go to the system's trash; and drags assets out to other apps, which receive copies while the assets stay. Nothing else: no preview, no opening, no folders. An asset larger than the size limit is still added, but the developer is warned when it arrives and the view marks it for as long as it stays that large, because it goes into the repository. The File view shows the assets folder like the rest of `.weaver`, read-only.

Weaver reads of the assets only their names and sizes; it reads the content of an asset only to copy it, and never otherwise.

The developer names an asset to the agent by tagging it, `@` followed by its name, in what he writes to the agent: a message in Chat, in a job's conversation or in an Ask more conversation, and a design request. While he writes, a list suggests the assets that match, and every tag that names an existing asset shows highlighted. When he sends, the agent is also told, for each asset tagged, to read its file, without that line appearing in what the developer sees of the conversation; the reading itself is left to the agent. A tag that names no asset at that moment stays plain text and tells the agent nothing.
