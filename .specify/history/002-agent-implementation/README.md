
# Feature Development History 

## Initial commits using speckit

- Read context.md to understand the context of the project
      ```text
      Read the .specify/memory/context.md file to understand the background of Kaiak project we are working on in this
      directory.
      ```

- Run speckit.specify to establish spec for the first skeleton feature
      ```text
      With Kaiak's foundational components now in place, the next objective is to integrate the Goose Agent (github.com/block/goose) within agent.rs and successfully execute a comprehensive end-to-end test. This will demonstrate our capability to run the Goose agent with provided incidents and receive messages, tool calls, and other outputs via streaming. This feature will be designated as 'agent-implementation'. Verification will be conducted on Kaiak as a standalone system, without IDE extension integration.
      ```

- Run speckit.plan to generate a technical plan
      ```text
      While implementing this feature, we must recognize that there are differences between how the IDE extension expects messages and how the Goose agent handles them. We need to carefully understand these differences. Some advanced features in the Goose agent are not currently supported by the IDE extension. We should note such details so that we can add those features to the IDE extension in the future. For example, sessions are not supported in the IDE, but they are an important feature, so we will add them to the IDE extension.
      ```

- Run speckit.tasks

- Run speckit.analyze

- Run speckit.implement


### Follow up fixes / addressing review
