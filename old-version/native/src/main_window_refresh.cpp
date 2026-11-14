#include "main_window.hpp"
#include "integrations/bridge_client.hpp"
#include "integrations/bridge_schemas.hpp"

#include <QApplication>
#include <QJsonDocument>
#include <QStatusBar>
#include <QTimer>

namespace {

// A cached snapshot younger than this is shown without asking the CLI again.
constexpr qint64 FreshReadMs = 3000;

QString readKey(const QStringList &arguments)
{
    return arguments.join(QChar(0x1f));
}

bool autoRefreshArea(const QString &area)
{
    static const QStringList areas = {QStringLiteral("overview"), QStringLiteral("project"),
        QStringLiteral("agents"), QStringLiteral("processes"), QStringLiteral("resources")};
    return areas.contains(area);
}

} // namespace

void MainWindow::refreshArea()
{
    const QStringList arguments = readCommandForArea(m_currentArea);
    if (arguments.isEmpty()) {
        appendOutput(QStringLiteral("Esta vista se integrará con el bridge compartido de ChxChx."));
        return;
    }
    // Areas that share a snapshot (Inicio, Proyecto, Agentes…) paint at once
    // from the cache and refresh in the background when it is not fresh.
    const auto cached = m_readCache.constFind(readKey(arguments));
    if (cached != m_readCache.cend()) {
        applyBridgePayload(cached->payload, m_currentArea);
        if (cached->fetchedAt.msecsTo(QDateTime::currentDateTimeUtc()) < FreshReadMs) return;
        startRead(arguments, true);
        return;
    }
    startRead(arguments, false);
}

void MainWindow::startRead(const QStringList &arguments, bool silent)
{
    if (m_readClient->isRunning()) {
        m_readQueued = true;
        return;
    }
    m_readArguments = arguments;
    m_readSilent = silent;
    m_readStartedGeneration = m_readGeneration;
    if (!m_readClient->execute(arguments)) return;
    if (!silent) statusBar()->showMessage(QStringLiteral("Consultando ChxChx…"));
}

void MainWindow::finishRead(int exitCode, QProcess::ExitStatus status)
{
    const QString output = QString::fromUtf8(m_readClient->readStandardOutput()).trimmed();
    const QString error = QString::fromUtf8(m_readClient->readStandardError()).trimmed();
    const auto json = QJsonDocument::fromJson(output.toUtf8());
    const bool succeeded = status == QProcess::NormalExit && exitCode == 0 && json.isObject();
    // An action finished while this read ran: its data may predate the change.
    const bool stale = m_readStartedGeneration != m_readGeneration;
    QString summary = output.isEmpty() ? error : output;
    if (succeeded && !stale) {
        const QJsonObject payload = json.object();
        if (payload.value(QStringLiteral("schema")).toString() != BridgeSchemas::Error)
            m_readCache.insert(readKey(m_readArguments), {payload, QDateTime::currentDateTimeUtc()});
        if (readCommandForArea(m_currentArea) == m_readArguments)
            summary = applyBridgePayload(payload, m_currentArea);
    }
    if (!m_readSilent || !succeeded) {
        appendOutput(summary);
        statusBar()->clearMessage();
    }
    if (m_readQueued || stale) {
        m_readQueued = false;
        QTimer::singleShot(0, this, &MainWindow::refreshArea);
    }
}

void MainWindow::autoRefreshTick()
{
    // Keep live views current without spending CPU while Studio is in the background.
    if (!isActiveWindow() || isMinimized() || !autoRefreshArea(m_currentArea)) return;
    // Refilling selectors would close an open drop-down or dialog under the user.
    if (m_readClient->isRunning() || QApplication::activeModalWidget() != nullptr
        || QApplication::activePopupWidget() != nullptr) return;
    const QStringList arguments = readCommandForArea(m_currentArea);
    if (!arguments.isEmpty()) startRead(arguments, true);
}

void MainWindow::invalidateReadCache()
{
    m_readCache.clear();
    ++m_readGeneration;
}
