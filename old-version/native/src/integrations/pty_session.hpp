#pragma once

#include <QObject>
#include <QByteArray>
#include <QString>
#include <QStringList>

#include <memory>

class PtySession final : public QObject {
    Q_OBJECT

public:
    explicit PtySession(QObject *parent = nullptr);
    ~PtySession() override;

    bool start(const QString &program, const QStringList &arguments,
        const QString &workingDirectory, int columns, int rows, QString *error = nullptr);
    bool write(const QByteArray &data);
    bool resize(int columns, int rows);
    void stop();
    bool isRunning() const;

signals:
    void outputReceived(const QByteArray &data);
    void processFinished(int exitCode);
    void failed(const QString &message);

private:
    struct State;
    std::unique_ptr<State> m_state;
};
