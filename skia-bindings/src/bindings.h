#ifndef SKIA_BINDINGS_BINDINGS_H
#define SKIA_BINDINGS_BINDINGS_H

#include "include/core/SkRefCnt.h"
#include "include/core/SkString.h"
#include <vector>

template<typename T>
inline sk_sp<T> spFromConst(const T* pt) {
    return sk_sp<T>(const_cast<T*>(pt));
}

template<typename T>
inline sk_sp<T> sp(T* pt) {
    return sk_sp<T>(pt);
}

// Used in textlayout::Paragraph::findTypefaces()
struct SkStrings {
    std::vector<SkString> strings;
};

#endif //SKIA_BINDINGS_BINDINGS_H
