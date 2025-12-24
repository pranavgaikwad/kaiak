
# Feature Development History 

## Initial commits using speckit

- Read context.md to understand the context of the project
      ```md
      Read the .specify/memory/context.md file to understand the background of Kaiak project we are working on in this
      directory.
      ```

- Run speckit.specify to establish spec for the first skeleton feature
      ```md
      I have added the Goose project to your context at ~/Projects/goose/, cloned from `https://github.com/block/goose`. Goose is an open-source, highly customizable, and flexible AI agent intended for general-purpose coding. Additionally, I have provided the directory ~/Projects/editor-extensions/vscode/core, containing a VSCode extension we developed to help users modernize and migrate their source code to newer technologies using AI agents. This IDE extension uses a static source code analysis tool to scan the currently open workspace and detect migration issues. The original intention was for Kaiak to serve as a Rust server that directly exposes the Goose AI Agent. However, we inadvertently implemented some features incorrectly. I now want to revisit the features we added to Kaiak and refactor them to resolve concerns with the original design. Here are the specific issues we want to address:

      - The current API flow requires a complete overhaul. The IDE will include a lightweight TypeScript wrapper that replaces the agentic/ module. This wrapper will launch the Kaiak server, stream messages to the webview, and relay any user interactions from the webview back to the agent. These interactions mainly consist of approvals or denials for tool calls, or any free-form text input provided by the user. The TypeScript layer will handle all communication with Kaiak. Here is how Kaiak should integrate into the overall IDE workflow:
        ```md
            1. kaiak_ts launches the Kaiak server
            2. kaiak_ts sends a configure() request to set up workspace configuration, model provider configuration, and any other required settings; Kaiak responds with any errors encountered
            3. kaiak_ts sends a generate_fix(session_id, incidents?) request to run the Goose agent
                - Kaiak streams tool calls, AI responses, and any other relevant data from Goose back to kaiak_ts
                - The call returns errors or nil depending on whether the Goose agent was successful
            5. kaiak_ts may optionally send delete(session_id) to remove specific session IDs
            6. kaiak_ts can call configure() multiple times to apply new configurations
        ```
         These are the only public API endpoints that Kaiak should expose. We should review our APIs and contracts and remove any unnecessary components.

      - Kaiak currently exposes functions for creating and deleting sessions and manages its own session lifecycle, but this is not ideal. Instead, we should leverage Goose's session APIs, essentially making Kaiak’s session management a thin wrapper around Goose’s session logic. Refer to goose::session::{SessionManager, SessionType} to understand session handling in Goose. Example usage:
         ```rust
            let session = SessionManager::create_session(
                    working_dir,
                    "Custom Agent Session".to_string(),
                    SessionType::User,
                ).await?;
         ```
         There is no need to expose any session operations publicly. The IDE will always provide a session ID with its requests. We may still need a minimal session module to track sessions in use, but we should not persist session data ourselves, relying instead on Goose’s session management.

      - Additionally, we are not initializing or configuring the Goose agent correctly. The correct flow for generate_fix() and agent setup is:
         ```md
            1. generate_fix(session_id, incidents?) is invoked.
            2. If the session exists, use it; otherwise, create a new session.
            3. Create and run the agent, streaming responses back to the caller.
         ```
         To set up the agent, consult goose::agents::{Agent, AgentEvent, SessionConfig}. When configuring and running the agent, ensure that:
         - All default Goose tools are available and our existing tool permissions are enforced,
         - It is easy to add custom tools to Goose,
         - The planning mode in Goose can be configured as needed.

      While we make these changes, we have to understand any impacts on related components such as user interactions etc should be taken into consideration. 
      ```

- Run speckit.clarify to add any clarification

- Run speckit.plan to generate a technical plan
      ```md
      Add explicit research tasks for deepdiving `goose::session` & `goose::agents` apis. We will not change any existing protocol & transport implementation for the server. Give special attention to updating existing docs, tests that are affected by the refactor. Re-use as many apis as you can from Goose. 
      ```

- Run speckit.tasks
   - Added some additional context during generating models:
      ```md
      - Remove max_file_size from workspace config
      - many of the ModelConfig fields will be passed by the IDE which we will simply pass on to GOose. so can we just re-use Goose's model config instead? 
      - For the permission config, i think we should keep a single hashmap that stores permissions for each tool, the permissions could be an enum. refer to existing api for reference.
      - Migration incident type only needs rule_id, id, message, description, effort, severity, fields no other fields are needed
      - Remove risk_assessment from InteractionContext
      - GenerateFixResponse doesn't look right. didnt we decide to wait when generate_fix is called until agent finishes. So it doesn't make sense to have estimated_duration. instead, a completed_at field would be better.

      ```

- Run speckit.analyze

- Run speckit.implement


### Follow up fixes / addressing review

