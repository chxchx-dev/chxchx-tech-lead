#include "../src/project_file_index.hpp"

#include <QCoreApplication>
#include <QDir>
#include <QEventLoop>
#include <QFile>
#include <QJsonObject>
#include <QTemporaryDir>
#include <QTimer>

namespace {

bool writeFile(const QString &path, const QByteArray &content)
{
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly)) return false;
    return file.write(content) == content.size();
}

bool waitForIndex(ProjectFileIndex &index)
{
    if (index.isReady()) return true;
    QEventLoop loop;
    QTimer timeout;
    timeout.setSingleShot(true);
    QObject::connect(&index, &ProjectFileIndex::indexChanged, &loop,
        [&loop](int, bool ready) { if (ready) loop.quit(); });
    QObject::connect(&timeout, &QTimer::timeout, &loop, &QEventLoop::quit);
    timeout.start(5000);
    loop.exec();
    return index.isReady();
}

} // namespace

int main(int argc, char *argv[])
{
    QCoreApplication app(argc, argv);
    QTemporaryDir project;
    if (!project.isValid()) return 1;
    const QString sourcePath = project.filePath(QStringLiteral("src/guide.txt"));
    if (!QDir().mkpath(project.filePath(QStringLiteral("src")))
        || !QDir().mkpath(project.filePath(QStringLiteral("node_modules/pkg")))
        || !QDir().mkpath(project.filePath(QStringLiteral(".git")))) return 2;
    if (!writeFile(sourcePath, "first line\nNeedle appears here\nlast line\n")
        || !writeFile(project.filePath(QStringLiteral("node_modules/pkg/ignored.txt")), "needle\n")
        || !writeFile(project.filePath(QStringLiteral(".git/ignored.txt")), "needle\n")
        || !writeFile(project.filePath(QStringLiteral("src/binary.bin")), QByteArray("\0needle", 7))) return 3;

    ProjectFileIndex index(project.path());
    if (!waitForIndex(index)) return 4;

    QJsonArray matches;
    bool truncated = false;
    bool finished = false;
    QEventLoop loop;
    QTimer timeout;
    timeout.setSingleShot(true);
    QObject::connect(&index, &ProjectFileIndex::contentSearchFinished, &loop,
        [&](const QString &query, const QJsonArray &results, bool wasTruncated) {
            if (query != QStringLiteral("needle")) return;
            matches = results;
            truncated = wasTruncated;
            finished = true;
            loop.quit();
        });
    QObject::connect(&timeout, &QTimer::timeout, &loop, &QEventLoop::quit);
    index.searchContent(QStringLiteral("needle"));
    timeout.start(5000);
    loop.exec();

    if (!finished || truncated || matches.size() != 1) return 5;
    const QJsonObject match = matches.first().toObject();
    if (match.value(QStringLiteral("path")).toString() != sourcePath) return 6;
    if (match.value(QStringLiteral("line")).toInt() != 2) return 7;
    if (!match.value(QStringLiteral("excerpt")).toString().contains(QStringLiteral("Needle"))) return 8;
    return 0;
}
