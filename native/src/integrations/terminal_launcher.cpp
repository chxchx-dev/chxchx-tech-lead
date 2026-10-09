#include "terminal_launcher.hpp"

#include <QProcess>
#include <QStandardPaths>

namespace {

#if defined(Q_OS_MACOS)
QString shellQuote(const QString &value)
{
    QString escaped = value;
    escaped.replace(QLatin1Char('\''), QStringLiteral("'\\''"));
    return QLatin1Char('\'') + escaped + QLatin1Char('\'');
}

QString shellCommand(const QStringList &arguments)
{
    QStringList quoted;
    quoted.reserve(arguments.size());
    for (const auto &argument : arguments) {
        quoted << shellQuote(argument);
    }
    return quoted.join(QLatin1Char(' '));
}
#endif

bool startDetached(const QString &program, const QStringList &arguments,
    const QString &workingDirectory, QString *error)
{
    qint64 processId = 0;
    if (QProcess::startDetached(program, arguments, workingDirectory, &processId)) {
        return true;
    }
    if (error != nullptr) {
        *error = QStringLiteral("No pude abrir el emulador de terminal: %1").arg(program);
    }
    return false;
}

} // namespace

bool launchExternalTerminal(const QStringList &command, const QString &workingDirectory, QString *error)
{
    if (command.isEmpty()) {
        if (error != nullptr) {
            *error = QStringLiteral("No hay un comando para ejecutar en la terminal.");
        }
        return false;
    }

    QStringList resolvedCommand = command;
    const QString executable = QStandardPaths::findExecutable(command.first());
    if (!executable.isEmpty()) {
        resolvedCommand[0] = executable;
    }

#if defined(Q_OS_WIN)
    const QString terminal = QStandardPaths::findExecutable(QStringLiteral("wt.exe"));
    if (terminal.isEmpty()) {
        if (error != nullptr) {
            *error = QStringLiteral("No encontré Windows Terminal (wt.exe), necesario para abrir una terminal aparte.");
        }
        return false;
    }
    QStringList arguments = {QStringLiteral("new-tab"), QStringLiteral("--startingDirectory"), workingDirectory,
        QStringLiteral("--")};
    arguments.append(resolvedCommand);
    return startDetached(terminal, arguments, workingDirectory, error);
#elif defined(Q_OS_MACOS)
    const QString osascript = QStandardPaths::findExecutable(QStringLiteral("osascript"));
    if (osascript.isEmpty()) {
        if (error != nullptr) {
            *error = QStringLiteral("No encontré osascript para abrir Terminal.app.");
        }
        return false;
    }
    const QString script = QStringLiteral(
        "on run argv\n"
        "  set projectPath to item 1 of argv\n"
        "  set shellCommand to item 2 of argv\n"
        "  tell application \"Terminal\" to do script (\"cd \" & quoted form of projectPath & \" && \" & shellCommand)\n"
        "end run");
    const QStringList arguments = {QStringLiteral("-e"), script, workingDirectory, shellCommand(resolvedCommand)};
    return startDetached(osascript, arguments, workingDirectory, error);
#else
    struct TerminalCandidate {
        const char *program;
        QStringList prefix;
    };
    const QList<TerminalCandidate> terminals = {
        {"konsole", {QStringLiteral("-e")}},
        {"kgx", {QStringLiteral("--")}},
        {"gnome-terminal", {QStringLiteral("--")}},
        {"wezterm", {QStringLiteral("start"), QStringLiteral("--cwd"), workingDirectory, QStringLiteral("--")}},
        {"alacritty", {QStringLiteral("-e")}},
        {"kitty", {QStringLiteral("-e")}},
        {"foot", {QStringLiteral("-e")}},
        {"x-terminal-emulator", {QStringLiteral("-e")}},
        {"xterm", {QStringLiteral("-e")}},
    };
    for (const auto &candidate : terminals) {
        const QString program = QStandardPaths::findExecutable(QString::fromUtf8(candidate.program));
        if (program.isEmpty()) {
            continue;
        }
        QStringList arguments = candidate.prefix;
        arguments.append(resolvedCommand);
        return startDetached(program, arguments, workingDirectory, error);
    }
    if (error != nullptr) {
        *error = QStringLiteral("No encontré una terminal compatible (Konsole, GNOME Console, Alacritty, Kitty o XTerm).");
    }
    return false;
#endif
}
