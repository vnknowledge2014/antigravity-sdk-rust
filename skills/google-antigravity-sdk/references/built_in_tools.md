# Built-in Tools Reference

In the `LocalAgentConfig` (used for local development), all built-in tools are
**enabled** by default. However, `run_command` is **denied** by the default
`confirm_run_command()` policy — all other tools are allowed. See
[Safety Policies](safety_policies.md) to customize this behavior.

The following table lists all built-in tools available in the SDK and their
descriptions.

Tool Enum                     | Tool Name          | Description
----------------------------- | ------------------ | --------------------------
`BuiltinTools::ListDir`       | `list_directory`   | List directory contents.
`BuiltinTools::SearchDir`     | `search_directory` | Search within directories.
`BuiltinTools::FindFile`      | `find_file`        | Find files by name.
`BuiltinTools::ViewFile`      | `view_file`        | View file contents.
`BuiltinTools::Finish`         | `finish`           | Finish and return output.
`BuiltinTools::CreateFile`    | `create_file`      | Create a new file.
`BuiltinTools::EditFile`      | `edit_file`        | Edit an existing file.
`BuiltinTools::RunCommand`    | `run_command`      | Execute a shell command.
`BuiltinTools::AskQuestion`   | `ask_question`     | Ask user a question.
`BuiltinTools::StartSubagent` | `start_subagent`   | Invoke a subagent.
`BuiltinTools::GenerateImage` | `generate_image`   | Generate or edit images.

> [!NOTE] Some production backends may require additional environment or
> filesystem configuration to support these tools.
