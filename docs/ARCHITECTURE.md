# useRA — Application Architecture

## 1. Architectural Principles

useRA follows a modular architecture with clear separation between:

- Presentation layer.
- Application logic.
- Domain models.
- Linux system integration.

UI components must not execute system commands directly.

## 2. Directory Structure

```
src/
├── main.rs
├── app.rs
│
├── ui/
│   ├── mod.rs
│   ├── layout.rs
│   ├── theme.rs
│   ├── icons.rs
│   ├── components/
│   │   ├── mod.rs
│   │   ├── details.rs
│   │   ├── form_modal.rs
│   │   ├── header.rs
│   │   ├── modal.rs
│   │   ├── panel.rs
│   │   ├── search_bar.rs
│   │   ├── sidebar.rs
│   │   ├── statusbar.rs
│   │   └── table.rs
│   │
│   └── pages/
│       ├── mod.rs
│       ├── dashboard.rs
│       ├── users.rs
│       ├── groups.rs
│       ├── sessions.rs
│       ├── resources.rs
│       └── settings.rs
│
├── events/
│   ├── mod.rs
│   ├── handler.rs
│   └── keymap.rs
│
├── models/
│   ├── mod.rs
│   ├── host.rs
│   ├── user.rs
│   ├── resource.rs
│   ├── group.rs
│   └── session.rs
│
├── services/
│   ├── mod.rs
│   ├── app_data.rs
│   ├── user_service.rs
│   ├── resource_service.rs
│   └── group_service.rs
│
├── system/
│   ├── mod.rs
│   ├── account_reader.rs
│   ├── acl.rs
│   ├── audit.rs
│   ├── command.rs
│   ├── filesystem.rs
│   ├── permission.rs
│   ├── privilege.rs
│   └── updater.rs
│
└── errors/
    ├── mod.rs
    └── app_error.rs
```

## 3. Application Layers

### Presentation Layer

Responsible for:

- Rendering terminal UI.
- Layout management.
- User interaction.
- Keyboard navigation.
- Dialogs and notifications.

Technology: Ratatui.

### Application Layer

Responsible for:

- Processing application actions.
- Coordinating services.
- Validating operations.
- Managing application state.

### Domain Layer

Responsible for:

- User models.
- Group models.
- Session models.
- Domain validation rules.

### System Integration Layer

Responsible for:

- Reading Linux account information.
- Executing administrative operations.
- Checking system privileges.
- Handling system command errors.

## 4. Data Flow

```
Keyboard Input
      |
      v
Event Handler
      |
      v
Application State
      |
      v
Application Service
      |
      v
Linux System Layer
      |
      v
System Result
      |
      v
Application State Update
      |
      v
Ratatui Rendering
```

## 5. Error Handling

Use a centralized application error type.

Errors should distinguish:

- Permission denied.
- Invalid input.
- User not found.
- Group not found.
- Command execution failure.
- System operation failure.
- Terminal initialization failure.

Never silently ignore failed system operations.

## 6. Testing

Use unit tests for:

- Input validation.
- User and group models.
- Application state.
- Service logic.

Use integration tests for Linux system operations with isolated test accounts and groups.

Avoid destructive tests against real system accounts.