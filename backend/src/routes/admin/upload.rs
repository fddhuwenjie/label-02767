use rocket::{Route, post, State};
use rocket::data::{Data, ToByteUnit};
use rocket::serde::json::Json;
use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![admin_upload_image]
}

#[post("/upload/image", data = "<data>")]
pub async fn admin_upload_image(
    _admin: AdminUser,
    content_type: &rocket::http::ContentType,
    data: Data<'_>,
) -> Json<ApiResponse<String>> {
    let ext = if content_type.is_jpeg() {
        "jpg"
    } else if content_type.is_png() {
        "png"
    } else if content_type.is_gif() {
        "gif"
    } else if content_type.is_svg() {
        "svg"
    } else {
        return Json(ApiResponse::error("不支持的图片格式，仅支持 JPG/PNG/GIF/SVG"));
    };

    let filename = format!("{}_{}.{}", chrono::Utc::now().format("%Y%m%d%H%M%S"), uuid::Uuid::new_v4().simple(), ext);
    let upload_dir = std::path::Path::new("static/uploads");
    if !upload_dir.exists() {
        if let Err(_) = std::fs::create_dir_all(upload_dir) {
            return Json(ApiResponse::error("创建上传目录失败"));
        }
    }

    let file_path = upload_dir.join(&filename);
    match data.open(10.mebibytes()).into_file(&file_path).await {
        Ok(f) if f.is_complete() => {
            let url = format!("/static/uploads/{}", filename);
            Json(ApiResponse::success(url, "上传成功"))
        }
        _ => Json(ApiResponse::error("文件上传失败或超过10MB限制")),
    }
}
