/// Skia skparagraph Module C Wrapper Functions

#include "bindings.h"

#include "modules/skparagraph/include/DartTypes.h"
#include "modules/skparagraph/include/FontCollection.h"
#include "modules/skparagraph/include/Metrics.h"
#include "modules/skparagraph/include/ParagraphCache.h"
#include "modules/skparagraph/include/Paragraph.h"
#include "modules/skparagraph/include/ParagraphBuilder.h"
#include "modules/skparagraph/include/ParagraphStyle.h"
#include "modules/skparagraph/include/TextShadow.h"
#include "modules/skparagraph/include/TextStyle.h"
#include "modules/skparagraph/include/TypefaceFontProvider.h"

// m84: needs definition of SkFontData
#include "src/core/SkFontDescriptor.h"

using namespace skia::textlayout;

//
// FontCollection.h
//

extern "C" {
    FontCollection* C_FontCollection_new() {
        return new FontCollection();
    }

    void C_FontCollection_setAssetFontManager(FontCollection* self, const SkFontMgr* fontManager) {
        self->setAssetFontManager(spFromConst(fontManager));
    }

    void C_FontCollection_setDynamicFontManager(FontCollection* self, const SkFontMgr* fontManager) {
        self->setDynamicFontManager(spFromConst(fontManager));
    }

    void C_FontCollection_setTestFontManager(FontCollection* self, const SkFontMgr* fontManager) {
        self->setTestFontManager(spFromConst(fontManager));
    }

    void C_FontCollection_setDefaultFontManager(FontCollection* self, const SkFontMgr* fontManager) {
        self->setDefaultFontManager(spFromConst(fontManager));
    }

    void C_FontCollection_setDefaultFontManager2(FontCollection* self, const SkFontMgr* fontManager, const char* defaultFamilyName) {
        self->setDefaultFontManager(spFromConst(fontManager), defaultFamilyName);
    }

    void C_FontCollection_setDefaultFontManager3(FontCollection* self, const SkFontMgr* fontManager, const SkStrings* familyNames) {
        self->setDefaultFontManager(spFromConst(fontManager), familyNames->strings);
    }

    SkFontMgr* C_FontCollection_getFallbackManager(const FontCollection* self) {
        return self->getFallbackManager().release();
    }

    void C_FontCollection_findTypefaces(FontCollection* self, const SkStrings* familyNames, SkFontStyle fontStyle, VecSink<sk_sp<SkTypeface>>* typefaces) {
        auto tfs = self->findTypefaces(familyNames->strings, fontStyle);
        typefaces->set(tfs);
    }

    SkTypeface* C_FontCollection_defaultFallback(FontCollection* self, SkUnichar unicode, SkFontStyle fontStyle, const SkString* locale) {
        return self->defaultFallback(unicode, fontStyle, *locale).release();
    }

    SkTypeface* C_FontCollection_defaultFallback2(FontCollection* self) {
        return self->defaultFallback().release();
    }

    bool C_FontCollection_fontFallbackEnabled(const FontCollection* self) {
        return const_cast<FontCollection*>(self)->fontFallbackEnabled();
    }

    ParagraphCache* C_FontCollection_paragraphCache(FontCollection* self) {
        return self->getParagraphCache();
    }
}

//
// ParagraphCache.h
//

extern "C" {
    void C_ParagraphCache_destruct(ParagraphCache* self) {
        self->~ParagraphCache();
    }

    int C_ParagraphCache_count(ParagraphCache* self) {
        return self->count();
    }
}

//
// ParagraphStyle.h
//

extern "C" {
    void C_StrutStyle_Construct(StrutStyle* uninitialized) {
        new(uninitialized) StrutStyle();
    }

    void C_StrutStyle_CopyConstruct(StrutStyle* uninitialized, const StrutStyle* other) {
        new(uninitialized) StrutStyle(*other);
    }
    
    void C_StrutStyle_destruct(StrutStyle* self) {
        self->~StrutStyle();
    }

    const SkString* C_StrutStyle_getFontFamilies(const StrutStyle* self, size_t* count) {
        auto& v = self->getFontFamilies();
        *count = v.size();
        return v.data();
    }

    void C_StrutStyle_setFontFamilies(StrutStyle* self, const SkString* data, size_t count) {
        self->setFontFamilies(std::vector<SkString>(data, data + count));
    }

    bool C_StrutStyle_equals(const StrutStyle* self, const StrutStyle* rhs) {
        return *self == *rhs;
    }
}

extern "C" {
    ParagraphStyle* C_ParagraphStyle_New() {
        return new ParagraphStyle();
    }

    ParagraphStyle* C_ParagraphStyle_NewCopy(const ParagraphStyle* other) {
        return new ParagraphStyle(*other);
    }

    void C_ParagraphStyle_delete(ParagraphStyle* self) {
        delete self;
    }

    bool C_ParagraphStyle_Equals(const ParagraphStyle* left, const ParagraphStyle* right) {
        return *left == *right;
    }

    bool C_ParagraphStyle_ellipsized(const ParagraphStyle* self) {
        return self->ellipsized();
    }
}

//
// TextShadow.h
//

extern "C" {
    bool C_TextShadow_Equals(const TextShadow* self, const TextShadow* other) {
        return *self == *other;
    }
}

//
// Metrics.h
//

extern "C" {
    void C_LineMetrics_destruct(LineMetrics* self) {
        self->~LineMetrics();
    }
    
    size_t C_LineMetrics_fLineMetrics_count(const LineMetrics* self, size_t begin, size_t end) {
        auto lower = self->fLineMetrics.lower_bound(begin);
        auto upper = self->fLineMetrics.upper_bound(end);
        return std::distance(lower, upper);
    }
    
    struct StyleMetricsRecord {
        size_t index;
        const StyleMetrics* metrics;
    };
    
    void C_LineMetrics_fLineMetrics_getRange(const LineMetrics* self, size_t begin, size_t end, StyleMetricsRecord* array) {
        auto lower = self->fLineMetrics.lower_bound(begin);
        auto upper = self->fLineMetrics.upper_bound(end);
        for (auto it = lower; it != upper; it++)
        {
            *array++ = StyleMetricsRecord { it->first, &it->second };
        }
    }
}

//
// Paragraph.h
//

extern "C" {
    void C_Paragraph_delete(Paragraph* self) {
        delete self;
    }
    
    void C_Paragraph_layout(Paragraph* self, SkScalar width) {
        self->layout(width);
    }

    void C_Paragraph_paint(Paragraph* self, SkCanvas* canvas, SkScalar x, SkScalar y) {
        self->paint(canvas, x, y);
    }

    void C_Paragraph_getRectsForRange(Paragraph *self, unsigned start, unsigned end, RectHeightStyle rectHeightStyle,
                                            RectWidthStyle rectWidthStyle, VecSink<TextBox>* textBoxes) {
        auto v = self->getRectsForRange(start, end, rectHeightStyle, rectWidthStyle);
        textBoxes->set(v);
    }

    void C_Paragraph_getRectsForPlaceholders(Paragraph* self, VecSink<TextBox>* result) {
        auto v = self->getRectsForPlaceholders();
        result->set(v);
    }

    void C_Paragraph_getGlyphPositionAtCoordinate(Paragraph* self, SkScalar x, SkScalar y, PositionWithAffinity* position) {
        *position = self->getGlyphPositionAtCoordinate(x, y);
    }

    void C_Paragraph_getWordBoundary(Paragraph* self, unsigned offset, size_t range[2]) {
        auto sk_range = self->getWordBoundary(offset);
        range[0] = sk_range.start;
        range[1] = sk_range.end;
    }

    void C_Paragraph_getLineMetrics(Paragraph* self, VecSink<LineMetrics>* result) {
        std::vector<LineMetrics> vec;
        self->getLineMetrics(vec);
        result->set(vec);
    }

    size_t C_Paragraph_lineNumber(Paragraph* self) {
        return self->lineNumber();
    }

    void C_Paragraph_markDirty(Paragraph* self) {
        self->markDirty();
    }
    
    int32_t C_Paragraph_unresolvedGlyphs(Paragraph* self) {
        return self->unresolvedGlyphs();
    }
}

//
// ParagraphBuilder.h
//

extern "C" {
    void C_ParagraphBuilder_delete(ParagraphBuilder* self) {
        delete self;
    }

    void C_ParagraphBuilder_pushStyle(ParagraphBuilder* self, const TextStyle* style) {
        self->pushStyle(*style);
    }

    void C_ParagraphBuilder_pop(ParagraphBuilder* self) {
        self->pop();
    }

    void C_ParagraphBuilder_peekStyle(ParagraphBuilder* self, TextStyle* style) {
        *style = self->peekStyle();
    }

    void C_ParagraphBuilder_addText(ParagraphBuilder* self, const char* text, size_t len) {
        self->addText(text, len);
    }

    void C_ParagraphBuilder_addPlaceholder(ParagraphBuilder* self, const PlaceholderStyle* placeholderStyle) {
        self->addPlaceholder(*placeholderStyle);
    }

    void C_ParagraphBuilder_setParagraphStyle(ParagraphBuilder* self, const ParagraphStyle* style) {
        self->setParagraphStyle(*style);
    }

    Paragraph* C_ParagraphBuilder_Build(ParagraphBuilder* self) {
        return self->Build().release();
    }

    ParagraphBuilder* C_ParagraphBuilder_make(const ParagraphStyle* style, const FontCollection* fontCollection) {
        return ParagraphBuilder::make(*style, spFromConst(fontCollection)).release();
    }
}

//
// TextStyle.h
//

extern "C" {
    void C_TextStyle_Types(const Block*, const Placeholder*) {}

    void C_FontFeature_CopyConstruct(FontFeature* uninitialized, const FontFeature* other) {
        new(uninitialized) FontFeature(*other);
    }

    void C_FontFeature_destruct(FontFeature* self) {
        self->~FontFeature();
    }

    void C_TextStyle_Construct(TextStyle* uninitialized) {
        new(uninitialized) TextStyle();
    }

    void C_TextStyle_CopyConstruct(TextStyle* uninitialized, const TextStyle* other) {
        new(uninitialized) TextStyle(*other);
    }

    void C_TextStyle_destruct(TextStyle* self) {
        self->~TextStyle();
    }

    const TextShadow* C_TextStyle_getShadows(const std::vector<TextShadow>* self, size_t* len_ref) {
        auto len = self->size();
        *len_ref = len;
        return len ? self->data() : nullptr;
    }

    void C_TextStyle_addShadow(TextStyle* self, const TextShadow* shadow) {
        self->addShadow(*shadow);
    }

    void C_TextStyle_resetShadows(TextStyle* self) {
        self->resetShadows();
    }

    const FontFeature* C_TextStyle_getFontFeatures(const std::vector<FontFeature>* self, size_t* len_ref) {
        auto size = self->size();
        *len_ref = size;
        return size ? self->data() : nullptr;
    }

    void C_TextStyle_addFontFeature(TextStyle* self, const SkString* fontFeature, int value) {
        self->addFontFeature(*fontFeature, value);
    }

    void C_TextStyle_resetFontFeatures(TextStyle* self) {
        self->resetFontFeatures();
    }
    
    const SkString* C_TextStyle_getFontFamilies(const TextStyle* self, size_t* count) {
        auto& v = self->getFontFamilies();
        *count = v.size();
        return v.data();
    }

    void C_TextStyle_setFontFamilies(TextStyle* self, const SkString* data, size_t count) {
        self->setFontFamilies(std::vector<SkString>(data, data + count));
    }

    void C_TextStyle_setTypeface(TextStyle* self, SkTypeface* typeface) {
        self->setTypeface(sk_sp<SkTypeface>(typeface));
    }
}

//
// TypefaceFontProvider
//

extern "C" {
    TypefaceFontStyleSet* C_TypefaceFontStyleSet_new(const SkString* family_name) {
        return new TypefaceFontStyleSet(*family_name);
    }

    void C_TypefaceFontStyleSet_appendTypeface(TypefaceFontStyleSet* self, SkTypeface* typeface) {
        self->appendTypeface(sk_sp<SkTypeface>(typeface));
    }
}

extern "C" {
    TypefaceFontProvider* C_TypefaceFontProvider_new() {
        return new TypefaceFontProvider();
    }

    size_t C_TypefaceFontProvider_registerTypeface(TypefaceFontProvider* self, SkTypeface* typeface, const SkString* alias) {
        if (alias) {
            return self->registerTypeface(sk_sp<SkTypeface>(typeface), *alias);
        }
        return self->registerTypeface(sk_sp<SkTypeface>(typeface));
    }
}
