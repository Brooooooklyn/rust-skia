pub mod pdf {
    use crate::interop::{self, DynamicMemoryWStream, SetStr};
    use crate::prelude::*;
    use crate::{scalar, DateTime, Document};
    use interop::AsStr;
    use skia_bindings as sb;
    use skia_bindings::{SkPDF_AttributeList, SkPDF_Metadata, SkPDF_StructureElementNode};
    use std::{ffi::CString, mem, ptr, slice};

    pub use sb::SkPDF_DocumentStructureType as DocumentStructureType;
    #[test]
    fn document_structure_type_naming() {
        let _ = DocumentStructureType::BibEntry;
    }

    pub type AttributeList = Handle<SkPDF_AttributeList>;
    unsafe impl Send for AttributeList {}
    unsafe impl Sync for AttributeList {}

    impl NativeDrop for SkPDF_AttributeList {
        fn drop(&mut self) {
            unsafe { sb::C_SkPDF_AttributeList_destruct(self) }
        }
    }

    impl Default for AttributeList {
        fn default() -> Self {
            AttributeList::from_native_c(unsafe { SkPDF_AttributeList::new() })
        }
    }

    impl AttributeList {
        pub fn append_int(
            &mut self,
            owner: impl AsRef<str>,
            name: impl AsRef<str>,
            value: i32,
        ) -> &mut Self {
            let owner = CString::new(owner.as_ref()).unwrap();
            let name = CString::new(name.as_ref()).unwrap();
            unsafe {
                self.native_mut()
                    .appendInt(owner.as_ptr(), name.as_ptr(), value)
            }
            self
        }

        pub fn append_float(
            &mut self,
            owner: impl AsRef<str>,
            name: impl AsRef<str>,
            value: f32,
        ) -> &mut Self {
            let owner = CString::new(owner.as_ref()).unwrap();
            let name = CString::new(name.as_ref()).unwrap();
            unsafe {
                self.native_mut()
                    .appendFloat(owner.as_ptr(), name.as_ptr(), value)
            }
            self
        }

        pub fn append_string(
            &mut self,
            owner: impl AsRef<str>,
            name: impl AsRef<str>,
            value: impl AsRef<str>,
        ) -> &mut Self {
            let owner = CString::new(owner.as_ref()).unwrap();
            let name = CString::new(name.as_ref()).unwrap();
            let value = CString::new(value.as_ref()).unwrap();
            unsafe {
                self.native_mut()
                    .appendString(owner.as_ptr(), name.as_ptr(), value.as_ptr())
            }
            self
        }

        pub fn append_float_array(
            &mut self,
            owner: impl AsRef<str>,
            name: impl AsRef<str>,
            value: &[f32],
        ) -> &mut Self {
            let owner = CString::new(owner.as_ref()).unwrap();
            let name = CString::new(name.as_ref()).unwrap();
            unsafe {
                sb::C_SkPDF_AttributeList_appendFloatArray(
                    self.native_mut(),
                    owner.as_ptr(),
                    name.as_ptr(),
                    value.as_ptr(),
                    value.len(),
                )
            }
            self
        }

        pub fn append_string_array(
            &mut self,
            owner: impl AsRef<str>,
            name: impl AsRef<str>,
            value: &[impl AsRef<str>],
        ) -> &mut Self {
            let owner = CString::new(owner.as_ref()).unwrap();
            let name = CString::new(name.as_ref()).unwrap();
            let value: Vec<interop::String> = value.iter().map(interop::String::from_str).collect();

            unsafe {
                sb::C_SkPDF_AttributeList_appendStringArray(
                    self.native_mut(),
                    owner.as_ptr(),
                    name.as_ptr(),
                    value.native().as_ptr(),
                    value.len(),
                )
            }
            self
        }
    }

    #[repr(transparent)]
    #[derive(Debug)]
    pub struct StructureElementNode(ptr::NonNull<SkPDF_StructureElementNode>);

    impl NativeAccess<SkPDF_StructureElementNode> for StructureElementNode {
        fn native(&self) -> &SkPDF_StructureElementNode {
            unsafe { self.0.as_ref() }
        }
        fn native_mut(&mut self) -> &mut SkPDF_StructureElementNode {
            unsafe { self.0.as_mut() }
        }
    }

    impl Drop for StructureElementNode {
        fn drop(&mut self) {
            unsafe { sb::C_SkPDF_StructureElementNode_delete(self.native_mut()) }
        }
    }

    impl Default for StructureElementNode {
        fn default() -> Self {
            Self::new("")
        }
    }

    impl StructureElementNode {
        pub fn new(type_string: impl AsRef<str>) -> Self {
            let mut node = Self::default();
            node.set_type_string(type_string);
            node
        }

        pub fn set_type_string(&mut self, type_string: impl AsRef<str>) -> &mut Self {
            self.native_mut().fTypeString.set_str(type_string);
            self
        }

        pub fn type_string(&self) -> &str {
            self.native().fTypeString.as_str()
        }

        pub fn set_child_vector(
            &mut self,
            mut child_vector: Vec<StructureElementNode>,
        ) -> &mut Self {
            // strategy is to move them out by setting them to nullptr (drop() will handle a nullptr on the rust side)
            unsafe {
                sb::C_SkPDF_StructureElementNode_setChildVector(
                    self.native_mut(),
                    child_vector.as_mut_ptr() as _,
                    child_vector.len(),
                )
            }
            self
        }

        pub fn append_child(&mut self, node: StructureElementNode) -> &mut Self {
            unsafe {
                sb::C_SkPDF_StructElementNode_appendChild(self.native_mut(), node.0.as_ptr());
            }
            mem::forget(node);
            self
        }

        pub fn child_vector(&self) -> &[StructureElementNode] {
            let mut ptr = ptr::null_mut();
            let len =
                unsafe { sb::C_SkPDF_StructureElementNode_getChildVector(self.native(), &mut ptr) };
            if len == 0 {
                &[]
            } else {
                unsafe { slice::from_raw_parts(ptr as _, len) }
            }
        }

        pub fn set_node_id(&mut self, node_id: i32) -> &mut Self {
            self.native_mut().fNodeId = node_id;
            self
        }

        pub fn node_id(&self) -> i32 {
            self.native().fNodeId
        }

        pub fn attributes(&self) -> &AttributeList {
            AttributeList::from_native_ref(&self.native().fAttributes)
        }

        pub fn attributes_mut(&mut self) -> &mut AttributeList {
            AttributeList::from_native_ref_mut(&mut self.native_mut().fAttributes)
        }

        pub fn set_alt(&mut self, alt: impl AsRef<str>) -> &mut Self {
            self.native_mut().fAlt.set_str(alt);
            self
        }

        pub fn alt(&self) -> &str {
            self.native().fAlt.as_str()
        }

        pub fn set_lang(&mut self, lang: impl AsRef<str>) -> &mut Self {
            self.native_mut().fLang.set_str(lang);
            self
        }

        pub fn lang(&self) -> &str {
            self.native().fLang.as_str()
        }
    }

    #[derive(Debug, Default)]
    pub struct Metadata {
        pub title: String,
        pub author: String,
        pub subject: String,
        pub keywords: String,
        pub creator: String,
        pub producer: String,
        pub creation: Option<DateTime>,
        pub modified: Option<DateTime>,
        pub raster_dpi: Option<scalar>,
        pub pdfa: bool,
        pub encoding_quality: Option<i32>,
        // TODO: this is not supported yet
        structure_element_tree_root: Option<StructureElementNode>,
    }

    // TODO: SetNodeId

    pub fn new_document(metadata: Option<&Metadata>) -> Document {
        let mut md = InternalMetadata::default();
        if let Some(metadata) = metadata {
            let internal = md.native_mut();
            internal.fTitle.set_str(&metadata.title);
            internal.fAuthor.set_str(&metadata.author);
            internal.fSubject.set_str(&metadata.subject);
            internal.fKeywords.set_str(&metadata.keywords);
            internal.fCreator.set_str(&metadata.creator);
            internal.fProducer.set_str(&metadata.producer);
            if let Some(creation) = metadata.creation {
                internal.fCreation = creation.into_native();
            }
            if let Some(modified) = metadata.modified {
                internal.fModified = modified.into_native();
            }
            if let Some(raster_dpi) = metadata.raster_dpi {
                internal.fRasterDPI = raster_dpi;
            }
            internal.fPDFA = metadata.pdfa;
            if let Some(encoding_quality) = metadata.encoding_quality {
                internal.fEncodingQuality = encoding_quality
            }
            if let Some(_structure_element_tree_root) = &metadata.structure_element_tree_root {
                // TODO: How can we be sure that the tree root is not being dropped while the document is processed?
                unimplemented!("");
            }
        }

        // We enable harfbuzz font sub-setting in PDF documents if textlayout is enabled.
        #[cfg(feature = "textlayout")]
        crate::icu::init();

        // we can't move the memory stream around anymore as soon it's referred by
        // the document.
        let mut memory_stream = Box::pin(DynamicMemoryWStream::new());
        let document = RCHandle::from_ptr(unsafe {
            sb::C_SkPDF_MakeDocument(memory_stream.native_mut().base_mut(), md.native())
        })
        .unwrap();

        Document::new(memory_stream, document)
    }

    //
    // Helper for constructing the internal metadata struct and setting associated strings.
    //

    type InternalMetadata = Handle<SkPDF_Metadata>;
    impl NativeDrop for SkPDF_Metadata {
        fn drop(&mut self) {
            unsafe { sb::C_SkPDF_Metadata_destruct(self) }
        }
    }

    impl Default for Handle<SkPDF_Metadata> {
        fn default() -> Self {
            Self::construct(|pdf_md| unsafe { sb::C_SkPDF_Metadata_Construct(pdf_md) })
        }
    }
}

#[test]
fn create_attribute_list() {
    use pdf::AttributeList;
    let mut _al = AttributeList::default();
    _al.append_float_array("Owner", "Name", &[1.0, 2.0, 3.0]);
    _al.append_string_array("Owner", "Name", &["A", "B", "C"]);
}
