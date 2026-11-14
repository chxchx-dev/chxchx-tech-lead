#include "project_file_index.hpp"

#include <QCoreApplication>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTimer>

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <vector>

namespace {

double percentile(std::vector<double> values, double percentileValue)
{
    std::sort(values.begin(), values.end());
    const auto index = static_cast<std::size_t>(
        std::ceil(percentileValue * static_cast<double>(values.size())) - 1.0);
    return values.at(std::min(index, values.size() - 1));
}

bool waitForIndex(ProjectFileIndex &index, int timeoutMs)
{
    if (index.isReady()) return true;
    QEventLoop loop;
    QTimer timeout;
    timeout.setSingleShot(true);
    QObject::connect(&index, &ProjectFileIndex::indexChanged, &loop,
        [&loop](int, bool ready) { if (ready) loop.quit(); });
    QObject::connect(&timeout, &QTimer::timeout, &loop, &QEventLoop::quit);
    timeout.start(timeoutMs);
    loop.exec();
    return index.isReady();
}

bool waitForContentSearch(ProjectFileIndex &index, int timeoutMs, double &elapsedMs)
{
    QEventLoop loop;
    QTimer timeout;
    timeout.setSingleShot(true);
    bool finished = false;
    QElapsedTimer elapsed;
    QObject::connect(&index, &ProjectFileIndex::contentSearchFinished, &loop,
        [&loop, &finished](const QString &, const QJsonArray &, bool) {
            finished = true;
            loop.quit();
        });
    QObject::connect(&timeout, &QTimer::timeout, &loop, &QEventLoop::quit);
    elapsed.start();
    index.searchContent(QStringLiteral("chxchx-benchmark-no-match"));
    timeout.start(timeoutMs);
    loop.exec();
    elapsedMs = static_cast<double>(elapsed.nsecsElapsed()) / 1'000'000.0;
    return finished;
}

QJsonObject summary(const std::vector<double> &samples)
{
    return {{QStringLiteral("p50_ms"), percentile(samples, 0.50)},
        {QStringLiteral("p95_ms"), percentile(samples, 0.95)}};
}

} // namespace

int main(int argc, char *argv[])
{
    QCoreApplication app(argc, argv);
    const QStringList arguments = app.arguments();
    if (arguments.size() < 2 || arguments.size() > 3) {
        std::fprintf(stderr, "Usage: chxchx-project-file-index-bench ROOT [RUNS]\n");
        return 2;
    }

    const QString root = arguments.at(1);
    const int runs = arguments.size() == 3 ? arguments.at(2).toInt() : 10;
    if (runs < 2 || runs > 100) {
        std::fprintf(stderr, "RUNS must be between 2 and 100.\n");
        return 2;
    }

    std::vector<double> indexSamples;
    std::vector<double> pathSearchSamples;
    std::vector<double> contentSearchSamples;
    int fileCount = 0;
    for (int run = 0; run < runs; ++run) {
        QElapsedTimer elapsed;
        elapsed.start();
        ProjectFileIndex index(root);
        if (!waitForIndex(index, 60'000)) {
            std::fprintf(stderr, "Index did not become ready within 60 seconds.\n");
            return 1;
        }
        indexSamples.push_back(static_cast<double>(elapsed.nsecsElapsed()) / 1'000'000.0);
        fileCount = index.fileCount();

        QElapsedTimer pathSearch;
        pathSearch.start();
        for (int iteration = 0; iteration < 100; ++iteration)
            (void)index.search(QStringLiteral("chxchx-benchmark-no-match"));
        pathSearchSamples.push_back(static_cast<double>(pathSearch.nsecsElapsed()) / 100'000'000.0);

        double contentElapsed = 0.0;
        if (!waitForContentSearch(index, 60'000, contentElapsed)) {
            std::fprintf(stderr, "Content search did not finish within 60 seconds.\n");
            return 1;
        }
        contentSearchSamples.push_back(contentElapsed);
    }

    QJsonObject result;
    result.insert(QStringLiteral("root"), root);
    result.insert(QStringLiteral("runs"), runs);
    result.insert(QStringLiteral("indexed_files"), fileCount);
    result.insert(QStringLiteral("index_build"), summary(indexSamples));
    result.insert(QStringLiteral("path_search_mean_of_100_calls"), summary(pathSearchSamples));
    result.insert(QStringLiteral("content_search"), summary(contentSearchSamples));
    std::puts(QJsonDocument(result).toJson(QJsonDocument::Compact).constData());
    return 0;
}
