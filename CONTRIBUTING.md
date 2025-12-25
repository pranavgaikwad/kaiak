# Contributing

## Using spec-kit

This project was developed entirely using [spec-kit](https://github.com/github/spec-kit). We highly recommend using it to contribute to this project. Please follow the installation instructions for spec-kit and ensure it is installed on your system before proceeding. Once installed, you can start using it in this project, as it is already set up.

### Working on a bug / new feature using spec-kit

We have used spec-kit with Claude Code, but you can use it with any agent of your choice. The instructions in this document are tailored for Claude Code; however, you should be able to find alternatives for your assistant in the [spec-kit documentation](https://github.com/github/spec-kit).

To add a new feature or to fix a bug, follow this process:

1. Load project information into Claude's context by simply asking the AI to read [context.md](.specify/memory/context.md).

2. Run `/speckit.specify <detailed_requirement>` to begin carving out design. The requirement that you pass here should contain every detail you can imagine about the feature you want to build - data flow, api surfaces, user input, expected output, components, message passing, etc. Make sure you also mention the name of the feature branch you want to work on. The format we are using is `<number>-<feature_name>`.

3. Run `/speckit.clarify` to add any clarification to requirements.

4. Run `/speckit.plan <technical_details>` to create a technical plan. Include as many technical details you can in the prompt you pass to this command.

5. Run `/speckit.tasks <optional_details>` to create tasks from the plan. Specify any last minute changes you want to make as optional details.

6. Run `/speckit.analyze` to run a consistency check.

7. Run `/speckit.implement` to start implementing. If its a huge chunk of work, you can repeat `/speckit.implement` until all tasks have been completed. 

Transcripts of all feature discussions are kept in [.specify/history](.specify/history/). The transcript for [003-agent-api-refactor](.specify/history/003-agent-api-refactor/README.md) is especially useful if you want to learn how to write better prompts. In contrast, [001-kaiak-skeleton](.specify/history/001-kaiak-selection/README.md) was my first spec-kit attempt and serves as an example of less effective prompts for specification generation. Notice how the former includes more detailed and specific design goals.

> We highly encourage you to maintain your work transcripts in [.specify/history](.specify/history/). You can also draft your prompts in that file *before* running the actual commands, giving you a scratch pad to refine and perfect your requirements *before* submitting them to the assistant.