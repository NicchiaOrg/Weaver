# Project setup

Putting Weaver in a project folder, keeping up to date what it installs for the agent, and making sure omp is installed, recent enough and connected to a model provider.

A folder without a project offers to initialise it. Weaver then creates the `.trama` folder with an empty Model and installs, for the agent, the skills it carries, the skills generated from the Requirements, the definitions of omp's sub-agents and the instructions of omp's advisor. With an empty Model the developer chooses one of two paths: Start from Scratch, where he writes the setup request and the agent turns it, through an interview, into the setup Proposal that becomes the first Model with the Tasks that build the application; or Import from Code, where the agent describes the code already there as import Proposals. Either way the first Proposals are approved by themselves when they are handed over, because the developer already decided in the interview or the code already exists.

omp is required: no job and no General conversation starts without it. Weaver finds it, offers to install it and to connect it to a provider through the system terminal, keeps it updated, and checks it again by itself until it answers.
