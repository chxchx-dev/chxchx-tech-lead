#include "bridge_client.hpp"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>
#include <QStandardPaths>

namespace {

QString resolveCliProgram()
{
    const QString override = qEnvironmentVariable("CHXCHX_TECH_CLI");
    if (!override.isEmpty() && QFileInfo(override).isExecutable()) {
        return QFileInfo(override).absoluteFilePath();
    }

    QDir directory(QCoreApplication::applicationDirPath());
    for (int depth = 0; depth < 8; ++depth) {
#ifdef Q_OS_WIN
        const QString relative = QStringLiteral(".venv/Scripts/chxchx-tech.exe");
#else
        const QString relative = QStringLiteral(".venv/bin/chxchx-tech");
#endif
        const QFileInfo candidate(directory.filePath(relative));
        if (candidate.isExecutable()) {
            return candidate.absoluteFilePath();
        }
        if (!directory.cdUp()) {
            break;
        }
    }
    return QStandardPaths::findExecutable(QStringLiteral("chxchx-tech"));
}

} // namespace

BridgeClient::BridgeClient(QString workingDirectory, QObject *parent)
    : QObject(parent)
    , m_program(resolveCliProgram())
{
    m_process.setWorkingDirectory(workingDirectory);
    m_process.setProcessChannelMode(QProcess::SeparateChannels);
    QObject::connect(&m_process,
        qOverload<int, QProcess::ExitStatus>(&QProcess::finished),
        this,
        [this](int exitCode, QProcess::ExitStatus status) {
            emit finished(exitCode, status);
        });
    QObject::connect(&m_process, &QProcess::errorOccurred, this,
        [this](QProcess::ProcessError error) {
            if (error == QProcess::FailedToStart) {
                emit failedToStart(m_process.errorString());
            }
        });
}

bool BridgeClient::isRunning() const
{
    return m_process.state() != QProcess::NotRunning;
}

QString BridgeClient::program() const
{
    return m_program;
}

void BridgeClient::setWorkingDirectory(const QString &path)
{
    m_process.setWorkingDirectory(path);
}

bool BridgeClient::execute(const QStringList &arguments)
{
    if (isRunning()) {
        return false;
    }
    if (m_program.isEmpty()) {
        emit failedToStart(QStringLiteral(
            "No se encontró chxchx-tech. Añádelo al PATH, configura CHXCHX_TECH_CLI "
            "o abre Studio desde el entorno del proyecto."));
        return false;
    }
    m_process.setProgram(m_program);
    m_process.setArguments(arguments);
    m_process.start();
    return true;
}

QByteArray BridgeClient::readStandardOutput()
{
    return m_process.readAllStandardOutput();
}

QByteArray BridgeClient::readStandardError()
{
    return m_process.readAllStandardError();
}
