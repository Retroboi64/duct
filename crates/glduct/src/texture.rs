use crate::gl::*;
use crate::types::*;
use duct_core::*;
use std::ffi::c_void;

impl GlPipeline {
    pub(crate) fn create_texture_2d_impl(
        &mut self,
        width: u32,
        height: u32,
        format: DuctTextureFormat,
        data: Option<&[u8]>,
    ) -> Result<DuctTextureHandle, DuctResult> {
        let gl = match &self.gl {
            Some(g) => g,
            None => return Err(DuctResult::ErrorInvalidState),
        };

        let (internal_fmt, gl_fmt, gl_type) = match format {
            DuctTextureFormat::Rgba8 => (GL_RGBA8, GL_RGBA, GL_UNSIGNED_BYTE),
            DuctTextureFormat::Rgb8 => (GL_RGB8, GL_RGB, GL_UNSIGNED_BYTE),
            DuctTextureFormat::R8 => (GL_R8, GL_RED, GL_UNSIGNED_BYTE),
            DuctTextureFormat::Rgba16Float => (GL_RGBA16F, GL_RGBA, GL_HALF_FLOAT),
            DuctTextureFormat::Depth32Float => (GL_DEPTH_COMPONENT32F, GL_DEPTH_COMPONENT, GL_FLOAT),
        };

        let mut gl_id = 0u32;
        unsafe {
            (gl.GenTextures)(1, &mut gl_id);
            (gl.BindTexture)(GL_TEXTURE_2D, gl_id);
            if format == DuctTextureFormat::Depth32Float {
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST as i32);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST as i32);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE as i32);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE as i32);
            } else {
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR as i32);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR as i32);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_REPEAT as i32);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_REPEAT as i32);
            }

            if format == DuctTextureFormat::R8 || format == DuctTextureFormat::Rgb8 {
                (gl.PixelStorei)(GL_UNPACK_ALIGNMENT, 1);
            }

            let ptr = data.map(|d| d.as_ptr() as *const c_void).unwrap_or(std::ptr::null());
            (gl.TexImage2D)(
                GL_TEXTURE_2D,
                0,
                internal_fmt as i32,
                width as i32,
                height as i32,
                0,
                gl_fmt,
                gl_type,
                ptr,
            );
        }

        let handle = DuctTextureHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.textures.insert(handle, GlTextureRecord {
            gl_id,
            _width: width,
            _height: height,
            format,
        });

        Ok(handle)
    }

    pub(crate) fn update_texture_2d_impl(
        &mut self,
        handle: DuctTextureHandle,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(tex) = self.textures.get(&handle) {
            let (gl_fmt, gl_type) = match tex.format {
                DuctTextureFormat::Rgba8 => (GL_RGBA, GL_UNSIGNED_BYTE),
                DuctTextureFormat::Rgb8 => (GL_RGB, GL_UNSIGNED_BYTE),
                DuctTextureFormat::R8 => (GL_RED, GL_UNSIGNED_BYTE),
                DuctTextureFormat::Rgba16Float => (GL_RGBA, GL_HALF_FLOAT),
                DuctTextureFormat::Depth32Float => (GL_DEPTH_COMPONENT, GL_FLOAT),
            };
            unsafe {
                (gl.BindTexture)(GL_TEXTURE_2D, tex.gl_id);
                (gl.TexSubImage2D)(
                    GL_TEXTURE_2D,
                    0,
                    x as i32,
                    y as i32,
                    width as i32,
                    height as i32,
                    gl_fmt,
                    gl_type,
                    data.as_ptr() as *const c_void,
                );
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn destroy_texture_impl(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if let Some(tex) = self.textures.remove(&handle) {
            if let Some(gl) = &self.gl {
                let mut id = tex.gl_id;
                unsafe {
                    (gl.DeleteTextures)(1, &mut id);
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_texture_impl(&mut self, slot: u32, handle: DuctTextureHandle) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        if let Some(tex) = self.textures.get(&handle) {
            unsafe {
                (gl.ActiveTexture)(GL_TEXTURE0 + slot);
                (gl.BindTexture)(GL_TEXTURE_2D, tex.gl_id);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn generate_mipmaps_impl(&mut self, handle: DuctTextureHandle) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(tex) = self.textures.get(&handle) {
            unsafe {
                (gl.BindTexture)(GL_TEXTURE_2D, tex.gl_id);
                (gl.GenerateMipmap)(GL_TEXTURE_2D);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR as i32);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn set_texture_filter_impl(
        &mut self,
        handle: DuctTextureHandle,
        min: DuctFilterMode,
        mag: DuctFilterMode,
    ) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(tex) = self.textures.get(&handle) {
            let min_val = map_filter_mode(min, true);
            let mag_val = map_filter_mode(mag, false);
            unsafe {
                (gl.BindTexture)(GL_TEXTURE_2D, tex.gl_id);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, min_val as i32);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, mag_val as i32);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn set_texture_wrap_impl(
        &mut self,
        handle: DuctTextureHandle,
        wrap_s: DuctWrapMode,
        wrap_t: DuctWrapMode,
    ) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(tex) = self.textures.get(&handle) {
            let ws = map_wrap_mode(wrap_s);
            let wt = map_wrap_mode(wrap_t);
            unsafe {
                (gl.BindTexture)(GL_TEXTURE_2D, tex.gl_id);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, ws);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, wt);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn create_render_target_impl(
        &mut self,
        desc: &DuctRenderTargetDesc,
    ) -> Result<DuctRenderTargetHandle, DuctResult> {
        let gl = match self.gl {
            Some(g) => g,
            None => return Err(DuctResult::ErrorInvalidState),
        };

        let color_tex_handle = self.create_texture_2d(desc.width, desc.height, desc.format, None)?;
        let tex_gl_id = self.textures.get(&color_tex_handle).unwrap().gl_id;

        let mut fbo_id = 0u32;
        let mut depth_rbo = 0u32;
        let mut depth_texture_handle = DuctTextureHandle::INVALID;

        if desc.has_depth {
            match self.create_texture_2d(desc.width, desc.height, DuctTextureFormat::Depth32Float, None) {
                Ok(dh) => {
                    depth_texture_handle = dh;
                }
                Err(err) => {
                    let _ = self.destroy_texture(color_tex_handle);
                    return Err(err);
                }
            }
        }

        let depth_gl_id = if depth_texture_handle.is_valid() {
            self.textures.get(&depth_texture_handle).unwrap().gl_id
        } else {
            0
        };

        unsafe {
            (gl.GenFramebuffers)(1, &mut fbo_id);
            (gl.BindFramebuffer)(GL_FRAMEBUFFER, fbo_id);
            (gl.FramebufferTexture2D)(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                tex_gl_id,
                0,
            );

            if depth_gl_id != 0 {
                (gl.FramebufferTexture2D)(
                    GL_FRAMEBUFFER,
                    GL_DEPTH_ATTACHMENT,
                    GL_TEXTURE_2D,
                    depth_gl_id,
                    0,
                );
            }

            let status = (gl.CheckFramebufferStatus)(GL_FRAMEBUFFER);
            if status != GL_FRAMEBUFFER_COMPLETE {
                eprintln!("[glduct][ERROR] Framebuffer incomplete: 0x{:X}", status);
                (gl.BindFramebuffer)(GL_FRAMEBUFFER, 0);
                (gl.DeleteFramebuffers)(1, &mut fbo_id);
                if depth_texture_handle.is_valid() {
                    let _ = self.destroy_texture(depth_texture_handle);
                }
                if depth_rbo != 0 {
                    (gl.DeleteRenderbuffers)(1, &mut depth_rbo);
                }
                let _ = self.destroy_texture(color_tex_handle);
                return Err(DuctResult::ErrorInitializationFailed);
            }

            // Restore previous or default framebuffer
            let prev_fbo = if self.active_render_target.is_valid() {
                self.render_targets
                    .get(&self.active_render_target)
                    .map(|rt| rt.fbo_id)
                    .unwrap_or(0)
            } else {
                0
            };
            (gl.BindFramebuffer)(GL_FRAMEBUFFER, prev_fbo);
        }

        let handle = DuctRenderTargetHandle(self.next_handle_id);
        self.next_handle_id += 1;

        self.render_targets.insert(
            handle,
            GlRenderTargetRecord {
                fbo_id,
                color_texture_handle: color_tex_handle,
                depth_texture_handle,
                depth_rbo,
                width: desc.width,
                height: desc.height,
            },
        );

        Ok(handle)
    }

    pub(crate) fn destroy_render_target_impl(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        if let Some(rt) = self.render_targets.remove(&handle) {
            if self.active_render_target == handle {
                if let Some(gl) = &self.gl {
                    unsafe {
                        (gl.BindFramebuffer)(GL_FRAMEBUFFER, 0);
                        (gl.Viewport)(0, 0, self.viewport_width as i32, self.viewport_height as i32);
                    }
                }
                self.active_render_target = DuctRenderTargetHandle::INVALID;
            }

            if let Some(gl) = &self.gl {
                unsafe {
                    let mut fbo = rt.fbo_id;
                    (gl.DeleteFramebuffers)(1, &mut fbo);
                    if rt.depth_rbo != 0 {
                        let mut rbo = rt.depth_rbo;
                        (gl.DeleteRenderbuffers)(1, &mut rbo);
                    }
                }
            }
            if rt.depth_texture_handle.is_valid() {
                let _ = self.destroy_texture(rt.depth_texture_handle);
            }
            let _ = self.destroy_texture(rt.color_texture_handle);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_render_target_impl(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        if !handle.is_valid() {
            unsafe {
                (gl.BindFramebuffer)(GL_FRAMEBUFFER, 0);
                (gl.Viewport)(0, 0, self.viewport_width as i32, self.viewport_height as i32);
            }
            self.active_render_target = DuctRenderTargetHandle::INVALID;
            DuctResult::Success
        } else if let Some(rt) = self.render_targets.get(&handle) {
            let fbo = rt.fbo_id;
            let w = rt.width;
            let h = rt.height;
            unsafe {
                (gl.BindFramebuffer)(GL_FRAMEBUFFER, fbo);
                (gl.Viewport)(0, 0, w as i32, h as i32);
            }
            self.active_render_target = handle;
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn get_render_target_texture_impl(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctResult> {
        if let Some(rt) = self.render_targets.get(&handle) {
            Ok(rt.color_texture_handle)
        } else {
            Err(DuctResult::ErrorResourceNotFound)
        }
    }

    pub(crate) fn get_render_target_depth_texture_impl(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctResult> {
        if let Some(rt) = self.render_targets.get(&handle) {
            if rt.depth_texture_handle.is_valid() {
                Ok(rt.depth_texture_handle)
            } else {
                Err(DuctResult::ErrorResourceNotFound)
            }
        } else {
            Err(DuctResult::ErrorResourceNotFound)
        }
    }

    pub(crate) fn blit_render_target_impl(
        &mut self,
        src: DuctRenderTargetHandle,
        dst: DuctRenderTargetHandle,
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
        filter: DuctFilterMode,
    ) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        let src_fbo = if src.is_valid() {
            match self.render_targets.get(&src) {
                Some(rt) => rt.fbo_id,
                None => return DuctResult::ErrorResourceNotFound,
            }
        } else {
            0
        };

        let dst_fbo = if dst.is_valid() {
            match self.render_targets.get(&dst) {
                Some(rt) => rt.fbo_id,
                None => return DuctResult::ErrorResourceNotFound,
            }
        } else {
            0
        };

        let gl_filter = match filter {
            DuctFilterMode::Nearest => GL_NEAREST as u32,
            _ => GL_LINEAR,
        };

        unsafe {
            (gl.BindFramebuffer)(GL_READ_FRAMEBUFFER, src_fbo);
            (gl.BindFramebuffer)(GL_DRAW_FRAMEBUFFER, dst_fbo);
            (gl.BlitFramebuffer)(
                0,
                0,
                src_w as i32,
                src_h as i32,
                0,
                0,
                dst_w as i32,
                dst_h as i32,
                GL_COLOR_BUFFER_BIT,
                gl_filter,
            );
            // Restore active render target
            let active_fbo = if self.active_render_target.is_valid() {
                self.render_targets
                    .get(&self.active_render_target)
                    .map(|rt| rt.fbo_id)
                    .unwrap_or(0)
            } else {
                0
            };
            (gl.BindFramebuffer)(GL_FRAMEBUFFER, active_fbo);
        }

        DuctResult::Success
    }

    pub(crate) fn read_pixels_impl(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        out_rgba: &mut [u8],
    ) -> DuctResult {
        let required = (width as usize) * (height as usize) * 4;
        if out_rgba.len() < required {
            return DuctResult::ErrorInvalidParameter;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        unsafe {
            (gl.PixelStorei)(GL_PACK_ALIGNMENT, 1);
            (gl.ReadPixels)(
                x as i32,
                y as i32,
                width as i32,
                height as i32,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                out_rgba.as_mut_ptr() as *mut c_void,
            );
        }
        DuctResult::Success
    }

}
