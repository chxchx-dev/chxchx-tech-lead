#include "pty_session.hpp"

#include <QSocketNotifier>
#include <QTimer>

#include <algorithm>
#include <atomic>
#include <thread>

#ifdef Q_OS_WIN
#define NOMINMAX
#include <windows.h>
#else
#include <cerrno>
#include <cstdlib>
#include <cstring>
#include <fcntl.h>
#include <signal.h>
#include <sys/ioctl.h>
#include <sys/wait.h>
#include <unistd.h>
#ifdef Q_OS_MACOS
#include <util.h>
#else
#include <pty.h>
#endif
#endif

struct PtySession::State {
#ifdef Q_OS_WIN
    HPCON console = nullptr;
    HANDLE input = INVALID_HANDLE_VALUE;
    HANDLE output = INVALID_HANDLE_VALUE;
    HANDLE process = INVALID_HANDLE_VALUE;
    std::thread reader;
    std::atomic_bool running = false;
#else
    int master = -1;
    pid_t child = -1;
    QSocketNotifier *readNotifier = nullptr;
    QSocketNotifier *writeNotifier = nullptr;
    QTimer *watcher = nullptr;
    QByteArray pending;
    bool running = false;
#endif
};

#ifdef Q_OS_WIN
namespace {
QString quoteArg(const QString &arg)
{
    QString out = QStringLiteral("\"");
    int slashes = 0;
    for (const QChar ch : arg) {
        if (ch == QLatin1Char('\\')) { ++slashes; continue; }
        if (ch == QLatin1Char('"')) out += QString(slashes * 2 + 1, QLatin1Char('\\'));
        else out += QString(slashes, QLatin1Char('\\'));
        out += ch;
        slashes = 0;
    }
    out += QString(slashes * 2, QLatin1Char('\\')) + QLatin1Char('"');
    return out;
}
}
#endif

PtySession::PtySession(QObject *parent) : QObject(parent), m_state(std::make_unique<State>()) {}
PtySession::~PtySession() { stop(); }

bool PtySession::start(const QString &program, const QStringList &arguments,
    const QString &cwd, int columns, int rows, QString *error)
{
    if (m_state->running || program.isEmpty()) {
        if (error) *error = QStringLiteral("La sesión PTY ya está activa o falta el ejecutable.");
        return false;
    }
    stop();
    columns = std::clamp(columns, 2, 500);
    rows = std::clamp(rows, 1, 300);
#ifdef Q_OS_WIN
    HANDLE inputRead = INVALID_HANDLE_VALUE;
    HANDLE outputWrite = INVALID_HANDLE_VALUE;
    SECURITY_ATTRIBUTES security{sizeof(security), nullptr, TRUE};
    if (!CreatePipe(&inputRead, &m_state->input, &security, 0)
        || !CreatePipe(&m_state->output, &outputWrite, &security, 0)) {
        if (inputRead != INVALID_HANDLE_VALUE) CloseHandle(inputRead);
        if (outputWrite != INVALID_HANDLE_VALUE) CloseHandle(outputWrite);
        if (error) *error = QStringLiteral("No pude crear los pipes de ConPTY (%1).").arg(GetLastError());
        stop(); return false;
    }
    HRESULT hr = CreatePseudoConsole(COORD{static_cast<SHORT>(columns), static_cast<SHORT>(rows)},
        inputRead, outputWrite, 0, &m_state->console);
    CloseHandle(inputRead); CloseHandle(outputWrite);
    if (FAILED(hr)) {
        if (error) *error = QStringLiteral("CreatePseudoConsole falló (%1).").arg(hr);
        stop(); return false;
    }
    SIZE_T bytes = 0;
    InitializeProcThreadAttributeList(nullptr, 1, 0, &bytes);
    auto *attrs = static_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(HeapAlloc(GetProcessHeap(), 0, bytes));
    if (!attrs || !InitializeProcThreadAttributeList(attrs, 1, 0, &bytes)
        || !UpdateProcThreadAttribute(attrs, 0, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
            m_state->console, sizeof(HPCON), nullptr, nullptr)) {
        if (attrs) HeapFree(GetProcessHeap(), 0, attrs);
        if (error) *error = QStringLiteral("No pude configurar ConPTY (%1).").arg(GetLastError());
        stop(); return false;
    }
    QStringList words{program}; words.append(arguments);
    QString command;
    for (const auto &word : words) {
        if (!command.isEmpty()) command += QLatin1Char(' ');
        command += quoteArg(word);
    }
    std::wstring mutableCommand = command.toStdWString();
    const std::wstring directory = cwd.toStdWString();
    STARTUPINFOEXW startup{}; startup.StartupInfo.cb = sizeof(startup); startup.lpAttributeList = attrs;
    PROCESS_INFORMATION info{};
    const BOOL ok = CreateProcessW(program.toStdWString().c_str(), mutableCommand.data(), nullptr, nullptr,
        FALSE, EXTENDED_STARTUPINFO_PRESENT, nullptr, directory.empty() ? nullptr : directory.c_str(),
        &startup.StartupInfo, &info);
    DeleteProcThreadAttributeList(attrs); HeapFree(GetProcessHeap(), 0, attrs);
    if (!ok) {
        if (error) *error = QStringLiteral("No pude iniciar el proceso ConPTY (%1).").arg(GetLastError());
        stop(); return false;
    }
    CloseHandle(info.hThread); m_state->process = info.hProcess; m_state->running = true;
    m_state->reader = std::thread([this] {
        char buffer[8192]; DWORD count = 0;
        while (ReadFile(m_state->output, buffer, sizeof(buffer), &count, nullptr) && count)
            emit outputReceived(QByteArray(buffer, static_cast<qsizetype>(count)));
        WaitForSingleObject(m_state->process, INFINITE);
        DWORD code = 1; GetExitCodeProcess(m_state->process, &code);
        m_state->running = false; emit processFinished(static_cast<int>(code));
    });
#else
    winsize size{}; size.ws_col = static_cast<unsigned short>(columns); size.ws_row = static_cast<unsigned short>(rows);
    QByteArray executable = program.toLocal8Bit();
    QList<QByteArray> args{executable};
    for (const auto &arg : arguments) args.append(arg.toLocal8Bit());
    QVector<char *> argv; argv.reserve(args.size() + 1);
    for (auto &arg : args) argv.append(arg.data());
    argv.append(nullptr);
    const pid_t child = forkpty(&m_state->master, nullptr, nullptr, &size);
    if (child < 0) {
        if (error) *error = QStringLiteral("forkpty falló (%1).").arg(QString::fromLocal8Bit(std::strerror(errno)));
        return false;
    }
    if (child == 0) {
        if (!cwd.isEmpty() && chdir(cwd.toLocal8Bit().constData()) != 0) _exit(126);
        setenv("TERM", "xterm-256color", 1);
        execvp(executable.constData(), argv.data());
        _exit(127);
    }
    const int masterFlags = fcntl(m_state->master, F_GETFL, 0);
    if (masterFlags >= 0) fcntl(m_state->master, F_SETFL, masterFlags | O_NONBLOCK);
    m_state->child = child; m_state->running = true;
    m_state->readNotifier = new QSocketNotifier(m_state->master, QSocketNotifier::Read, this);
    connect(m_state->readNotifier, &QSocketNotifier::activated, this, [this] {
        char buffer[8192];
        for (;;) {
            const ssize_t n = ::read(m_state->master, buffer, sizeof(buffer));
            if (n > 0) emit outputReceived(QByteArray(buffer, static_cast<qsizetype>(n)));
            else if (n < 0 && errno == EINTR) continue;
            else {
                if (n == 0 || (n < 0 && errno != EAGAIN && errno != EWOULDBLOCK))
                    m_state->readNotifier->setEnabled(false);
                break;
            }
        }
    });
    m_state->writeNotifier = new QSocketNotifier(m_state->master, QSocketNotifier::Write, this);
    m_state->writeNotifier->setEnabled(false);
    connect(m_state->writeNotifier, &QSocketNotifier::activated, this, [this] {
        if (m_state->pending.isEmpty()) {
            m_state->writeNotifier->setEnabled(false);
            return;
        }
        const ssize_t n = ::write(m_state->master, m_state->pending.constData(),
            static_cast<std::size_t>(m_state->pending.size()));
        if (n > 0) m_state->pending.remove(0, static_cast<qsizetype>(n));
        m_state->writeNotifier->setEnabled(false);
        if (!m_state->pending.isEmpty()) {
            if (n < 0 && errno != EINTR && errno != EAGAIN && errno != EWOULDBLOCK) {
                const QString message = QStringLiteral("Falló la escritura al PTY (%1).").arg(errno);
                m_state->pending.clear();
                emit failed(message);
                return;
            }
            QTimer::singleShot(10, this, [this] {
                if (m_state->running && !m_state->pending.isEmpty())
                    m_state->writeNotifier->setEnabled(true);
            });
        }
    });
    m_state->watcher = new QTimer(this); m_state->watcher->setInterval(100);
    connect(m_state->watcher, &QTimer::timeout, this, [this] {
        int status = 0;
        if (m_state->child > 0 && waitpid(m_state->child, &status, WNOHANG) == m_state->child) {
            const int code = WIFEXITED(status) ? WEXITSTATUS(status) : 128 + WTERMSIG(status);
            m_state->child = -1; m_state->running = false; m_state->pending.clear(); emit processFinished(code);
        }
    });
    m_state->watcher->start();
#endif
    return true;
}

bool PtySession::write(const QByteArray &data)
{
    if (!m_state->running) return false;
#ifdef Q_OS_WIN
    DWORD written = 0;
    return WriteFile(m_state->input, data.constData(), static_cast<DWORD>(data.size()), &written, nullptr)
        && written == static_cast<DWORD>(data.size());
#else
    m_state->pending.append(data);
    if (m_state->writeNotifier) m_state->writeNotifier->setEnabled(true);
    return true;
#endif
}

bool PtySession::resize(int columns, int rows)
{
    if (!m_state->running) return false;
    columns = std::clamp(columns, 2, 500); rows = std::clamp(rows, 1, 300);
#ifdef Q_OS_WIN
    return SUCCEEDED(ResizePseudoConsole(m_state->console,
        COORD{static_cast<SHORT>(columns), static_cast<SHORT>(rows)}));
#else
    winsize size{}; size.ws_col = static_cast<unsigned short>(columns); size.ws_row = static_cast<unsigned short>(rows);
    return ioctl(m_state->master, TIOCSWINSZ, &size) == 0;
#endif
}

void PtySession::stop()
{
#ifdef Q_OS_WIN
    if (m_state->process != INVALID_HANDLE_VALUE) {
        if (m_state->running) TerminateProcess(m_state->process, 1);
        if (m_state->console) { ClosePseudoConsole(m_state->console); m_state->console = nullptr; }
        if (m_state->reader.joinable()) m_state->reader.join();
        CloseHandle(m_state->process); m_state->process = INVALID_HANDLE_VALUE;
    }
    if (m_state->input != INVALID_HANDLE_VALUE) { CloseHandle(m_state->input); m_state->input = INVALID_HANDLE_VALUE; }
    if (m_state->output != INVALID_HANDLE_VALUE) { CloseHandle(m_state->output); m_state->output = INVALID_HANDLE_VALUE; }
    if (m_state->console) { ClosePseudoConsole(m_state->console); m_state->console = nullptr; }
    m_state->running = false;
#else
    if (m_state->watcher) m_state->watcher->stop();
    if (m_state->readNotifier) m_state->readNotifier->setEnabled(false);
    if (m_state->writeNotifier) m_state->writeNotifier->setEnabled(false);
    if (m_state->child > 0) {
        const pid_t child = m_state->child;
        kill(-child, SIGHUP);
        kill(-child, SIGKILL);
        kill(child, SIGKILL);
        int status = 0;
        while (waitpid(child, &status, 0) < 0 && errno == EINTR) {}
    }
    m_state->child = -1;
    if (m_state->master >= 0) { ::close(m_state->master); m_state->master = -1; }
    delete m_state->readNotifier; m_state->readNotifier = nullptr;
    delete m_state->writeNotifier; m_state->writeNotifier = nullptr;
    delete m_state->watcher; m_state->watcher = nullptr;
    m_state->pending.clear(); m_state->running = false;
#endif
}

bool PtySession::isRunning() const { return m_state->running; }
