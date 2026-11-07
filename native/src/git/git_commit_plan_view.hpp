#pragma once

#include "git_commit_plan.hpp"

#include <QFrame>

class QLabel;
class QVBoxLayout;

// The strip in the Changes tab that lists the proposed commits and prepares them one at a time.
class GitCommitPlanView final : public QFrame {
    Q_OBJECT
public:
    explicit GitCommitPlanView(QWidget *parent = nullptr);
    void setPlan(const QList<GitCommitGroup> &plan, const QString &agentLabel);
    const QList<GitCommitGroup> &plan() const { return m_plan; }
    // Tests.
    int groupCount() const { return int(m_plan.size()); }
    void prepare(int index) { emit prepareRequested(index); }

signals:
    void prepareRequested(int index);
    void dismissed();

private:
    void rebuild();

    QList<GitCommitGroup> m_plan;
    QString m_agent;
    QLabel *m_title = nullptr;
    QVBoxLayout *m_rows = nullptr;
};
