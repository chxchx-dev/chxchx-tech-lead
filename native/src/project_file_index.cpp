#include "project_file_index.hpp"

#include <QDir>
#include <QFileInfo>
#include <QMetaObject>
#include <QPointer>
#include <QThread>

#include <algorithm>
#include <functional>
#include <utility>

namespace {

constexpr qsizetype MaxIndexedFiles = 100000;

bool excludedDirectory(const QString &name)
{
    static const QStringList excluded = {
        QStringLiteral(".git"), QStringLiteral(".venv"), QStringLiteral("venv"),
        QStringLiteral("node_modules"), QStringLiteral(".next"), QStringLiteral("dist"),
        QStringLiteral("build"), QStringLiteral("coverage"), QStringLiteral(".cache"),
        QStringLiteral(".turbo"), QStringLiteral("target"), QStringLiteral("__pycache__")
    };
    return excluded.contains(name, Qt::CaseInsensitive);
}

} // namespace

ProjectFileIndex::ProjectFileIndex(QString projectRoot, QObject *parent)
    : QObject(parent), m_projectRoot(QDir(std::move(projectRoot)).absolutePath())
{
    rebuild();
}

ProjectFileIndex::~ProjectFileIndex()
{
    if (m_cancel) m_cancel->store(true);
    if (m_thread) {
        if (m_thread->isRunning()) m_thread->wait();
        delete m_thread;
    }
}

bool ProjectFileIndex::isReady() const
{
    return m_ready;
}

int ProjectFileIndex::fileCount() const
{
    return static_cast<int>(m_paths.size());
}

QStringList ProjectFileIndex::search(const QString &query, int limit) const
{
    QStringList matches;
    const QString needle = query.trimmed();
    if (!m_ready || needle.isEmpty() || limit <= 0) return matches;
    const QDir root(m_projectRoot);
    for (const QString &path : m_paths) {
        const QString relative = root.relativeFilePath(path);
        if (relative.contains(needle, Qt::CaseInsensitive)) {
            matches.append(path);
            if (matches.size() >= limit) break;
        }
    }
    return matches;
}

void ProjectFileIndex::rebuild()
{
    if (m_thread && m_thread->isRunning()) return;
    if (m_thread) {
        m_thread->wait();
        m_thread = nullptr;
    }
    if (m_cancel) m_cancel->store(true);
    m_cancel = std::make_shared<std::atomic_bool>(false);
    const auto cancellation = m_cancel;
    const QString root = m_projectRoot;
    QPointer<ProjectFileIndex> target(this);
    m_ready = false;
    m_paths.clear();
    emit indexChanged(0, false);

    QThread *worker = QThread::create([target, cancellation, root] {
        QStringList paths;
        std::function<void(const QString &)> scan = [&](const QString &directoryPath) {
            if (cancellation->load() || paths.size() >= MaxIndexedFiles) return;
            const QFileInfoList entries = QDir(directoryPath).entryInfoList(
                QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden | QDir::System,
                QDir::Name | QDir::DirsFirst);
            for (const QFileInfo &entry : entries) {
                if (cancellation->load() || paths.size() >= MaxIndexedFiles) break;
                if (entry.isSymLink()) continue;
                if (entry.isDir()) {
                    if (!excludedDirectory(entry.fileName())) scan(entry.absoluteFilePath());
                } else if (entry.isFile()) {
                    paths.append(entry.absoluteFilePath());
                }
            }
        };
        scan(root);
        if (!cancellation->load() && target) {
            QMetaObject::invokeMethod(target, [target, paths = std::move(paths)]() mutable {
                if (target) target->acceptIndex(std::move(paths));
            }, Qt::QueuedConnection);
        }
    });
    m_thread = worker;
    connect(worker, &QThread::finished, this, [this, worker] {
        if (m_thread == worker) m_thread = nullptr;
        worker->deleteLater();
    });
    worker->start();
}

void ProjectFileIndex::acceptIndex(QStringList paths)
{
    std::sort(paths.begin(), paths.end(), [](const QString &left, const QString &right) {
        return left.compare(right, Qt::CaseInsensitive) < 0;
    });
    m_paths = std::move(paths);
    m_ready = true;
    emit indexChanged(fileCount(), true);
}
