use chrono::{DateTime, TimeZone, Utc};
use glib::translate::FromGlibPtrFull;
use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int};
use std::path;
use std::ptr;

mod ffi;
mod util;

#[derive(Debug)]
pub struct PopplerDocument(*mut ffi::PopplerDocument);

#[derive(Debug)]
pub struct PopplerPage(*mut ffi::PopplerPage);

impl PopplerDocument {
    pub fn new_from_file<P: AsRef<path::Path>>(
        p: P,
        password: &str,
    ) -> Result<PopplerDocument, glib::error::Error> {
        let pw = CString::new(password).map_err(|_| {
            glib::error::Error::new(
                glib::FileError::Inval,
                "Password invalid (possibly contains NUL characters)",
            )
        })?;

        let path_cstring = util::path_to_glib_url(p)?;
        let doc = util::call_with_gerror(|err_ptr| unsafe {
            ffi::poppler_document_new_from_file(path_cstring.as_ptr(), pw.as_ptr(), err_ptr)
        })?;

        Ok(PopplerDocument(doc))
    }
    pub fn new_from_data(
        data: &mut [u8],
        password: &str,
    ) -> Result<PopplerDocument, glib::error::Error> {
        if data.is_empty() {
            return Err(glib::error::Error::new(
                glib::FileError::Inval,
                "data is empty",
            ));
        }
        let pw = CString::new(password).map_err(|_| {
            glib::error::Error::new(
                glib::FileError::Inval,
                "Password invalid (possibly contains NUL characters)",
            )
        })?;

        let doc = util::call_with_gerror(|err_ptr| unsafe {
            ffi::poppler_document_new_from_data(
                data.as_mut_ptr() as *mut c_char,
                data.len() as c_int,
                pw.as_ptr(),
                err_ptr,
            )
        })?;

        Ok(PopplerDocument(doc))
    }
    pub fn get_title(&self) -> Option<String> {
        unsafe {
            let ptr: *mut c_char = ffi::poppler_document_get_title(self.0);
            if ptr.is_null() {
                None
            } else {
                CString::from_raw(ptr).into_string().ok()
            }
        }
    }
    pub fn get_metadata(&self) -> Option<String> {
        unsafe {
            let ptr: *mut c_char = ffi::poppler_document_get_metadata(self.0);
            if ptr.is_null() {
                None
            } else {
                CString::from_raw(ptr).into_string().ok()
            }
        }
    }
    pub fn get_pdf_version_string(&self) -> Option<String> {
        unsafe {
            let ptr: *mut c_char = ffi::poppler_document_get_pdf_version_string(self.0);
            if ptr.is_null() {
                None
            } else {
                CString::from_raw(ptr).into_string().ok()
            }
        }
    }
    pub fn get_permissions(&self) -> u8 {
        unsafe { ffi::poppler_document_get_permissions(self.0) as u8 }
    }

    pub fn get_n_pages(&self) -> usize {
        // FIXME: what's the correct type here? can we assume a document
        //        has a positive number of pages?
        (unsafe { ffi::poppler_document_get_n_pages(self.0) }) as usize
    }

    pub fn get_page(&self, index: usize) -> Option<PopplerPage> {
        match unsafe { ffi::poppler_document_get_page(self.0, index as c_int) } {
            ptr if ptr.is_null() => None,
            ptr => Some(PopplerPage(ptr)),
        }
    }

    pub fn get_creation_date(&self) -> Option<DateTime<Utc>> {
        unsafe {
            let timestamp = ffi::poppler_document_get_creation_date(self.0);
            if timestamp > 0 {
                Some(Utc.timestamp_opt(timestamp, 0).unwrap())
            } else {
                None
            }
        }
    }

    pub fn get_modification_date(&self) -> Option<DateTime<Utc>> {
        unsafe {
            let timestamp = ffi::poppler_document_get_modification_date(self.0);
            if timestamp > 0 {
                Some(Utc.timestamp_opt(timestamp, 0).unwrap())
            } else {
                None
            }
        }
    }
    pub fn set_creation_date(&self, creation_date: DateTime<Utc>) {
        unsafe {
            ffi::poppler_document_set_creation_date(self.0, creation_date.timestamp());
        }
    }
    pub fn set_modification_date(&self, modification_date: DateTime<Utc>) {
        unsafe {
            ffi::poppler_document_set_modification_date(self.0, modification_date.timestamp());
        }
    }

    pub fn clear_creation_date(&self) {
        unsafe {
            ffi::poppler_document_set_creation_date(self.0, -1);
        }
    }
    pub fn clear_modification_date(&self) {
        unsafe {
            ffi::poppler_document_set_modification_date(self.0, -1);
        }
    }

    pub fn save<P: AsRef<path::Path>>(&self, p: P) -> Result<(), glib::error::Error> {
        let path_cstring = util::path_to_glib_url(p)?;
        let uri = path_cstring.as_ptr();
        let err = ptr::null_mut();
        unsafe {
            let return_value =
                ffi::poppler_document_save(self.0, uri, err as *mut *mut glib::ffi::GError);

            if return_value == 0 {
                Err(glib::error::Error::from_glib_full(err))
            } else {
                Ok(())
            }
        }
    }
}

impl PopplerPage {
    pub fn get_size(&self) -> (f64, f64) {
        let mut width: f64 = 0.0;
        let mut height: f64 = 0.0;

        unsafe {
            ffi::poppler_page_get_size(
                self.0,
                &mut width as *mut f64 as *mut c_double,
                &mut height as *mut f64 as *mut c_double,
            )
        }

        (width, height)
    }

    pub fn render(&self, ctx: &cairo::Context) {
        let ctx_raw = ctx.to_raw_none();
        unsafe { ffi::poppler_page_render(self.0, ctx_raw) }
    }

    pub fn render_for_printing(&self, ctx: &cairo::Context) {
        let ctx_raw = ctx.to_raw_none();
        unsafe { ffi::poppler_page_render_for_printing(self.0, ctx_raw) }
    }

    pub fn get_text(&self) -> Option<&str> {
        match unsafe { ffi::poppler_page_get_text(self.0) } {
            ptr if ptr.is_null() => None,
            ptr => unsafe { Some(CStr::from_ptr(ptr).to_str().unwrap()) },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::PopplerDocument;
    use crate::PopplerPage;
    use cairo::Context;
    use cairo::Format;
    use cairo::ImageSurface;
    use chrono::{TimeZone, Utc};
    use std::{fs::File, io::Read};
    use tempfile::tempdir;

    #[test]
    fn test1() {
        let filename = "test.pdf";
        let doc = PopplerDocument::new_from_file(filename, "").unwrap();
        let num_pages = doc.get_n_pages();

        println!("Document has {} page(s)", num_pages);

        let surface = cairo::PdfSurface::new(420.0, 595.0, "output.pdf").unwrap();
        let ctx = Context::new(&surface).unwrap();

        // FIXME: move iterator to poppler
        for page_num in 0..num_pages {
            let page = doc.get_page(page_num).unwrap();
            let (w, h) = page.get_size();
            println!("page {} has size {}, {}", page_num, w, h);
            surface.set_size(w, h).unwrap();

            ctx.save().unwrap();
            page.render(&ctx);

            println!("Text: {:?}", page.get_text().unwrap_or(""));

            ctx.restore().unwrap();
            ctx.show_page().unwrap();
        }
        // g_object_unref (page);
        //surface.write_to_png("file.png");
        surface.finish();
    }

    #[test]
    fn test2_from_file() {
        let path = "test.pdf";
        let doc: PopplerDocument = PopplerDocument::new_from_file(path, "upw").unwrap();
        let num_pages = doc.get_n_pages();
        let title = doc.get_title().unwrap();
        let metadata = doc.get_metadata();
        let version_string = doc.get_pdf_version_string();
        let permissions = doc.get_permissions();
        let page: PopplerPage = doc.get_page(0).unwrap();
        let (w, h) = page.get_size();

        println!(
            "Document {} has {} page(s) and is {}x{}",
            title, num_pages, w, h
        );
        println!(
            "Version: {:?}, Permissions: {:x?}",
            version_string, permissions
        );

        assert!(metadata.is_some());
        assert_eq!(version_string, Some("PDF-1.3".to_string()));
        assert_eq!(permissions, 0xff);

        assert_eq!(title, "This is a test PDF file");

        let surface = ImageSurface::create(Format::ARgb32, w as i32, h as i32).unwrap();
        let ctx = Context::new(&surface).unwrap();

        ctx.save().unwrap();
        page.render(&ctx);
        ctx.restore().unwrap();
        ctx.show_page().unwrap();

        let mut f: File = File::create("out.png").unwrap();
        surface.write_to_png(&mut f).expect("Unable to write PNG");
    }
    #[test]
    fn test2_from_data() {
        let path = "test.pdf";
        let mut file = File::open(path).unwrap();
        let mut data: Vec<u8> = Vec::new();
        file.read_to_end(&mut data).unwrap();
        let doc: PopplerDocument = PopplerDocument::new_from_data(&mut data[..], "upw").unwrap();
        let num_pages = doc.get_n_pages();
        let title = doc.get_title().unwrap();
        let metadata = doc.get_metadata();
        let version_string = doc.get_pdf_version_string();
        let permissions = doc.get_permissions();
        let page: PopplerPage = doc.get_page(0).unwrap();
        let (w, h) = page.get_size();

        println!(
            "Document {} has {} page(s) and is {}x{}",
            title, num_pages, w, h
        );
        println!(
            "Version: {:?}, Permissions: {:x?}",
            version_string, permissions
        );

        assert!(metadata.is_some());
        assert_eq!(version_string, Some("PDF-1.3".to_string()));
        assert_eq!(permissions, 0xff);
    }

    #[test]
    fn test3() {
        let mut data = vec![];

        assert!(PopplerDocument::new_from_data(&mut data[..], "upw").is_err());
    }

    #[test]
    fn test_get_time() {
        let path = "test.pdf";
        let doc: PopplerDocument = PopplerDocument::new_from_file(path, "upw").unwrap();
        assert_eq!(
            doc.get_creation_date().unwrap().to_rfc3339(),
            "2000-06-28T23:21:08+00:00"
        );
        assert_eq!(
            doc.get_modification_date().unwrap().to_rfc3339(),
            "2013-10-28T19:24:13+00:00"
        );
    }

    #[test]
    fn test_set_time() {
        let path = "test.pdf";
        let doc: PopplerDocument = PopplerDocument::new_from_file(path, "upw").unwrap();
        let created = Utc.with_ymd_and_hms(2000, 1, 1, 12, 34, 56).unwrap();
        let modified = Utc.with_ymd_and_hms(2023, 4, 5, 23, 59, 59).unwrap();

        doc.set_creation_date(created);
        doc.set_modification_date(modified);

        let tempdir = tempdir().unwrap();
        let path2 = tempdir.path().join("test2.pdf");
        let tempfile = File::create(&path2).unwrap();

        doc.save(&path2).unwrap();

        let doc2: PopplerDocument = PopplerDocument::new_from_file(path2, "upw").unwrap();
        assert_eq!(doc2.get_creation_date().unwrap(), created);
        assert_eq!(doc2.get_modification_date().unwrap(), modified);

        drop(tempfile);
        tempdir.close().unwrap();
    }

    #[test]
    fn test_clear_time() {
        let path = "test.pdf";
        let doc: PopplerDocument = PopplerDocument::new_from_file(path, "upw").unwrap();

        doc.clear_creation_date();
        doc.clear_modification_date();

        let tempdir = tempdir().unwrap();
        let path2 = tempdir.path().join("test2.pdf");
        let tempfile = File::create(&path2).unwrap();

        doc.save(&path2).unwrap();

        let doc2: PopplerDocument = PopplerDocument::new_from_file(path2, "upw").unwrap();

        assert!(doc2.get_creation_date().is_none());
        assert!(doc2.get_modification_date().is_none());

        drop(tempfile);
        tempdir.close().unwrap();
    }
}
