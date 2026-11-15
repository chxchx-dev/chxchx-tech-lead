#pragma once

#include <QList>
#include <QString>

struct SourceSymbol {
    QString name;
    QString kind;
    int line = 0;
};

class SymbolIndex final {
public:
    static QList<SourceSymbol> scan(const QString &text, const QString &suffix);
};
