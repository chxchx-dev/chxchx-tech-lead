#include "../src/integrations/pty_session.hpp"

#include <QCoreApplication>
#include <QEventLoop>
#include <QTimer>

int main(int argc, char *argv[])
{
    QCoreApplication app(argc, argv);
    PtySession session;
    QByteArray output;
    int exitCode = -1;
    bool inputSent = false;
    QEventLoop loop;
    QTimer timeout;
    timeout.setSingleShot(true);
    QObject::connect(&timeout, &QTimer::timeout, &loop, &QEventLoop::quit);
    QObject::connect(&session, &PtySession::outputReceived, &loop,
        [&output, &inputSent, &session](const QByteArray &data) {
            output.append(data);
            if (!inputSent && output.contains("READY")) {
                const bool resized = session.resize(96, 30);
#ifdef Q_OS_WIN
                const QByteArray enter = "chxchx-input\r";
#else
                const QByteArray enter = "chxchx-input\n";
#endif
                inputSent = resized && session.write(enter);
            }
        });
    QObject::connect(&session, &PtySession::processFinished, &loop,
        [&loop, &exitCode](int code) { exitCode = code; loop.quit(); });

#ifdef Q_OS_WIN
    const bool started = session.start(QStringLiteral("powershell.exe"),
        {QStringLiteral("-NoLogo"), QStringLiteral("-NoProfile"), QStringLiteral("-Command"),
         QStringLiteral("$answer = Read-Host 'READY'; Write-Output ('PTY_OK=' + $answer)")}, {}, 80, 24);
#else
    const bool started = session.start(QStringLiteral("/bin/sh"),
        {QStringLiteral("-c"), QStringLiteral("stty -echo; printf READY; read line; printf 'PTY_OK=%s\\n' \"$line\"; stty size")}, {}, 80, 24);
#endif
    if (!started) return 1;
    timeout.start(5000);
    loop.exec();
    if (timeout.isActive()) timeout.stop();
    session.stop();
    if (exitCode != 0 || !output.contains("PTY_OK=chxchx-input") || !inputSent) return 2;
#ifndef Q_OS_WIN
    if (!output.contains("30 96")) return 3;
#endif
    return 0;
}
