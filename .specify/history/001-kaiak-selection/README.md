
# Feature Development History 

## Initial commits using speckit

- 1. Begin research
      ```text
      I have added the Goose project to your context at ~/Projects/goose/, which was cloned from 
      `https://github.com/block/goose`. Goose is an open-source, highly customizable, and flexible AI agent designed for general-purpose coding. I have also included the directory ~/Projects/editor-extensions/vscode/core, which contains a VSCode extension we developed to help users modernize and migrate their source code to newer technologies using AI agents. This IDE extension utilizes a static source code analysis tool to scan the currently open workspace and identify migration issues. Each occurrence of a migration issue in the source code is called an incident. Users can select a specific incident in a file, all issues within a file, or multiple incidents of the same issue across different files. The IDE then forwards this data to the external agentic/ module. You also have access to the agentic/ module’s source code at ~/Projects/editor-extensions/agentic/. Common APIs are provided by the shared/ module, included in the context as well. The agentic/ module implements a LangGraph agent that communicates with the vscode/core extension. The vscode/core extension manages the state of the agentic workflow and interfaces with the webview, where AI messages are displayed in a chat format. The AI workflow is not permitted to make file changes directly; instead, it sends a special message to the user and *waits* for user interaction to be resolved. The webview source code is located in ~/Projects/editor-extensions/webview-ui/.

      We intend to replace the agentic/ module with the Goose Agent. Your task is to thoroughly understand how agentic/, vscode/core, and webview-ui/ interact and how they work together. Next, review the Goose agent’s source code to understand its functionality and public APIs. If we were to replace agentic/ with the Goose Agent, could Goose be brought in as a dependency? How might we integrate Goose if it provides public APIs? Assess the features of our current IDE extension and determine if all existing features can be supported by Goose, such as streaming different types of messages, waiting for user interaction in the UI, displaying thought steps, etc.

      Note that our goal is not to implement a general-purpose code assistant; we seek a controlled experience specifically aimed at modernizing and migrating source code with AI agents. Therefore, it’s essential that the chosen agent allows us to adjust prompts and add or modify tools as needed. We are not making any code changes at this stage—focus solely on discussing and designing the possible integration approach.
      ```

- 2. Run speckit.constitution to establish baseline
      ```text
      Now that you are familiar with the problem statement, let’s begin work on this Rust server, which we will refer to as “Kaiak” from now on. We should focus on coding standards, code quality, testing practices, and above all, ensuring a consistent user experience. It is essential that our user experience remains user-friendly—debugging should be straightforward, progress should be clearly shown during long-running tasks, errors must be communicated effectively, and comprehensive logging should be in place. We will use GitHub Actions for continuous integration, with actions running on pull requests to execute tests. For testing, we will prioritize end-to-end and integration tests over smaller unit tests. However, more granular unit tests should still be written for complex functions or critical business logic within the codebase. Comments in the source code should be reserved for complex functions or important portions of the code, while unnecessary or verbose comments for trivial sections should be avoided.
      ```

- 3. Run speckit.specify to establish spec for the first skeleton feature
      ```text
      Let's begin developing the skeleton for our server, "Kaiak." Kaiak will be a standalone server designed to run the Goose agent and will support the following capabilities:

      1. Accept fix generation requests from the IDE extension for one or more incidents within the workspace.
      2. Run the Goose AI agent with customized prompts and/or tools—the incident information will be incorporated into the prompts provided to the agent.
      3. Manage the lifecycle of the agent.
      4. Stream AI messages back to the IDE.
      5. Process user input from the IDE's webview, including user interactions for tool calls, file modification requests, etc.

      The Goose AI agent will handle the core processing. With Kaiak, our primary objective is to enable migration use cases using Goose—we are not building a general-purpose coding assistant. Accordingly, the IDE extension should provide a controlled mechanism for performing migrations through Goose. We will leverage data from static analysis tools to identify migration issues and integrate this information into custom prompts. We will configure the Goose agent with specific tools and will stream all messages back to the user. A crucial requirement is that Goose will not be allowed to make any file changes directly. For example, the previous agentic module utilized a ModifiedFile message type to display proposed file modifications to the user and request confirmation. Similarly, we will stream tool calls, the agent's reasoning process, and all AI-generated messages back to the user.
      ```

- 4. Run speckit.plan to generate a technical plan
      ```text
      Kaiak will be a standalone server developed in Rust. It will utilize `Goose (github.com/block/goose)` as a dependency, leveraging its public APIs to create, manage, and execute the Goose AI agent. Communication with clients (such as IDEs) will occur via LSP-style JSON-RPC messages (including content length and type) over sockets, named pipes, or optionally stdio. Where feasible, Kaiak may be distributed as a WebAssembly module or binary executable. Continuous integration will be managed with GitHub Actions to enforce pull request checks, complemented by local scripts allowing developers to run the same CI workflows. We will prioritize end-to-end and integration testing over unit or small isolated tests. Kaiak will minimize external dependencies, while avoiding re-inventing solutions for well-established needs such as socket communication and JSON-RPC implementations compatible with VSCode’s protocol.
      ```

- 5. Run speckit.tasks

- 6. Run speckit.analyze

- 7. Run speckit.implement


### Follow up fixes / addressing review



```text
We have implemented our feature 001-kaiak-skeleton, establishing a complete Rust server capable of running the Goose agent.  Comprehensive specifications, plans, tasks, and related documents were created under the specs/001-kaiak-skeleton directory. I would now like to make some revisions to the feature based on feedback received during review:

## Major changes affecting multiple files
     
1. The IDE extension is supposed to pass provider settings to Kaiak. Internally, the IDE uses the standard langchain structs to store provider settings. We do not need to have any specific model specific structs or settings. We will pass the provider settings as is to the Goose agent. You will have to understand how Goose agent handles provider settings and make changes in our code accordingly. We do not need to validate the provider settings either. As a result, we do not need an environment variable for AI model as well. 

2. There's some gap in how the IDE expects messages / user interactions and how we have tied that all up with Goose. For instance, file modification approval thing is its own thing right now. Notice that apart from file modifications, we will also run tools and we need a way to approve or reject these tool calls as well. Right now the way things are set up, file modification approval seems to be its own thing, I think it would be better to derive it from a more generic tool call approval mechanism. Understand how Goose agent's tool calls map to message types expected by the IDE. Some specific reviews around this:
   - In the security.rs file, require_approval should take a map of tool names in which the default config should only
     have the modification disabled. For this, you might have to understand how Goose agent handles file modifications
More generally, I think we need to re-visit the contracts and models and ensure that we have a coherent api between Goose <-> Kaiak <-> IDE. 

3. We do not need the whole resource management module yet, its adding a bit of complexity to the codebase. Lets remove it for now.

Lastly, remove unnecessary verbose comments in the source code where trivial things are being done. Only add comments for complex functions or important parts of the code.

## Minor changes 

1. In security.rs, remove allowed_workspace_roots...we do not need this feature.

Thoroughly review each item of feedback, and identify the necessary code modifications as well as any updates required for existing specifications, documentation, contracts, etc. Ensure all changes are made appropriately. If any requirements are unclear, do not make assumptions; instead, ask for clarification.
```