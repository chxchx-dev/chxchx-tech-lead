#include "../src/integrations/pty_session.hpp"

#include <QCoreApplication>
#include <QEventLoop>
#include <QTimer>

#include <cstdio>

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
    QString startError;
    const QString commandProcessor = qEnvironmentVariable("COMSPEC", QStringLiteral("cmd.exe"));
    const bool started = session.start(commandProcessor,
        {QStringLiteral("/D"), QStringLiteral("/Q"), QStringLiteral("/V:ON"), QStringLiteral("/C"),
         QStringLiteral("set /p answer=READY: & echo PTY_OK=!answer!")}, {}, 80, 24, &startError);
#else
    const bool started = session.start(QStringLiteral("/bin/sh"),
        {QStringLiteral("-c"), QStringLiteral("stty -echo; printf READY; read line; printf 'PTY_OK=%s\\n' \"$line\"; stty size")}, {}, 80, 24);
#endif
    if (!started) {
#ifdef Q_OS_WIN
        std::fprintf(stderr, "PTY start failed: %s\n", qPrintable(startError));
#else
        std::fprintf(stderr, "PTY start failed.\n");
#endif
        return 1;
    }
    timeout.start(5000);
    loop.exec();
    if (timeout.isActive()) timeout.stop();
    session.stop();
    if (exitCode != 0 || !output.contains("PTY_OK=chxchx-input") || !inputSent) {
        std::fprintf(stderr, "PTY smoke failed: exit=%d inputSent=%d output=", exitCode, inputSent);
        std::fwrite(output.constData(), 1, static_cast<size_t>(output.size()), stderr);
        std::fputc('\n', stderr);
        return 2;
    }
#ifndef Q_OS_WIN
    if (!output.contains("30 96")) {
        std::fprintf(stderr, "PTY resize failed: output did not contain 30 96.\n");
        return 3;
    }
#endif
    return 0;
}
