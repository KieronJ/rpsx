use imgui::{
    ImGui,
    ImGuiCol,
    ImGuiColorEditFlags,
    ImGuiCond,
    ImString,
    ImVec4,
    Ui,
};

use crate::frontend::Frontend;
use crate::gpu_viewer::{GpuCommand, GpuFrame};
use crate::psx::System;
use crate::{Options, Scaling};

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const RED: [f32; 3] = [1.0, 0.0, 0.0];

const RED_OVERLAY: [f32; 4] = [1.0, 0.0, 0.0, 0.25];
const GREEN_OVERLAY: [f32; 4] = [0.0, 1.0, 0.0, 0.25];

pub struct Gui {
    pub imgui: ImGui,
    pub renderer: Renderer,
}

impl Gui {
    pub fn new(display: &Display) -> Self {
        let mut imgui = ImGui::init();
        imgui.set_ini_filename(None);

        let style = imgui.style_mut();
        style.window_rounding = 2.0;
        style.child_rounding = 2.0;
        style.popup_rounding = 2.0;
        style.frame_rounding = 2.0;
        style.scrollbar_rounding = 2.0;
        style.grab_rounding = 2.0;

        style.colors[ImGuiCol::Border as usize] = ImVec4::new(0.43, 0.43, 0.50, 0.50);
        style.colors[ImGuiCol::FrameBg as usize] = ImVec4::new(0.43, 0.43, 0.50, 0.50);
        style.colors[ImGuiCol::FrameBgHovered as usize] = ImVec4::new(0.98, 0.37, 0.27, 0.40);
        style.colors[ImGuiCol::FrameBgActive as usize] = ImVec4::new(0.98, 0.37, 0.27, 0.67);
        style.colors[ImGuiCol::TitleBg as usize] = ImVec4::new(0.04, 0.04, 0.04, 1.00);
        style.colors[ImGuiCol::TitleBgActive as usize] = ImVec4::new(0.75, 0.29, 0.21, 1.00);
        style.colors[ImGuiCol::TitleBgCollapsed as usize] = ImVec4::new(0.00, 0.00, 0.00, 0.51);
        style.colors[ImGuiCol::MenuBarBg as usize] = ImVec4::new(0.14, 0.14, 0.14, 1.00);
        style.colors[ImGuiCol::ScrollbarBg as usize] = ImVec4::new(0.02, 0.02, 0.02, 0.53);
        style.colors[ImGuiCol::ScrollbarGrab as usize] = ImVec4::new(0.31, 0.31, 0.31, 1.00);
        style.colors[ImGuiCol::ScrollbarGrabHovered as usize] = ImVec4::new(0.41, 0.41, 0.41, 1.00);
        style.colors[ImGuiCol::ScrollbarGrabActive as usize] = ImVec4::new(0.51, 0.51, 0.51, 1.00);
        style.colors[ImGuiCol::CheckMark as usize] = ImVec4::new(0.98, 0.37, 0.27, 1.00);
        style.colors[ImGuiCol::SliderGrab as usize] = ImVec4::new(0.88, 0.33, 0.24, 1.00);
        style.colors[ImGuiCol::SliderGrabActive as usize] = ImVec4::new(0.98, 0.37, 0.27, 1.00);
        style.colors[ImGuiCol::Button as usize] = ImVec4::new(1.00, 0.39, 0.28, 0.40);
        style.colors[ImGuiCol::ButtonHovered as usize] = ImVec4::new(0.26, 0.59, 0.98, 1.00);
        style.colors[ImGuiCol::ButtonActive as usize] = ImVec4::new(0.06, 0.53, 0.98, 1.00);
        style.colors[ImGuiCol::Header as usize] = ImVec4::new(0.26, 0.59, 0.98, 0.31);
        style.colors[ImGuiCol::HeaderHovered as usize] = ImVec4::new(0.26, 0.59, 0.98, 0.80);
        style.colors[ImGuiCol::HeaderActive as usize] = ImVec4::new(0.26, 0.59, 0.98, 1.00);
        style.colors[ImGuiCol::Separator as usize] = ImVec4::new(0.43, 0.43, 0.50, 0.50);
        style.colors[ImGuiCol::SeparatorHovered as usize] = ImVec4::new(0.10, 0.40, 0.75, 0.78);
        style.colors[ImGuiCol::SeparatorActive as usize] = ImVec4::new(0.10, 0.40, 0.75, 1.00);
        style.colors[ImGuiCol::ResizeGrip as usize] = ImVec4::new(0.26, 0.59, 0.98, 0.25);
        style.colors[ImGuiCol::ResizeGripHovered as usize] = ImVec4::new(0.26, 0.59, 0.98, 0.67);
        style.colors[ImGuiCol::ResizeGripActive as usize] = ImVec4::new(0.26, 0.59, 0.98, 0.95);
        style.colors[ImGuiCol::PlotLines as usize] = ImVec4::new(0.61, 0.61, 0.61, 1.00);
        style.colors[ImGuiCol::PlotLinesHovered as usize] = ImVec4::new(1.00, 0.43, 0.35, 1.00);
        style.colors[ImGuiCol::PlotHistogram as usize] = ImVec4::new(0.90, 0.70, 0.00, 1.00);
        style.colors[ImGuiCol::PlotHistogramHovered as usize] = ImVec4::new(1.00, 0.60, 0.00, 1.00);
        style.colors[ImGuiCol::TextSelectedBg as usize] = ImVec4::new(0.98, 0.37, 0.27, 0.35);
        style.colors[ImGuiCol::DragDropTarget as usize] = ImVec4::new(1.00, 1.00, 0.00, 0.90);
        style.colors[ImGuiCol::NavHighlight as usize] = ImVec4::new(0.26, 0.59, 0.98, 1.00);
        style.colors[ImGuiCol::NavWindowingHighlight as usize] = ImVec4::new(1.00, 1.00, 1.00, 0.70);
        style.colors[ImGuiCol::NavWindowingDimBg as usize] = ImVec4::new(0.80, 0.80, 0.80, 0.20);
        style.colors[ImGuiCol::ModalWindowDimBg as usize] = ImVec4::new(0.80, 0.80, 0.80, 0.35);

        let renderer = Renderer::init(&mut imgui, display).unwrap();

        Self {
            imgui: imgui,
            renderer: renderer,
        }
    }

    pub fn draw(ui: &Ui,
                options: &mut Options,
                gpu_frame: &mut GpuFrame,
                system: &mut System,
                video: &VideoInterface) {
        let file = ui.menu(im_str!("File"));
        let emu = ui.menu(im_str!("Emulator"));
        let debug = ui.menu(im_str!("Debug"));
        let view = ui.menu(im_str!("View"));

        ui.main_menu_bar(|| {
            file.build(|| { Gui::draw_file_menu(ui, system, video); });
            emu.build(|| { Gui::draw_emu_menu(ui, options, system); });
            debug.build(|| { Gui::draw_debug_menu(ui, options); });
            view.build(|| { Gui::draw_view_menu(ui, options); });

            if options.draw_full_vram && options.draw_display_area {
                let (window_x, window_y) = video.get_size();
                let (x, y) = system.get_display_origin();
                let (w, h) = system.get_display_size();

                let x_scale = (window_x as f32) / 1024.0;
                let y_scale = (window_y as f32) / 512.0;

                let x1 = (x as f32) * x_scale;
                let x2 = ((x + w) as f32) * x_scale;
                let y1 = (y as f32) * y_scale;
                let y2 = ((y + h) as f32) * y_scale;

                let draw_list = ui.get_window_draw_list();

                draw_list.with_clip_rect([0.0, 0.0], [window_x as f32, window_y as f32], || {
                    draw_list.add_line([x1, y1], [x2, y1], RED).build();
                    draw_list.add_line([x1, y2], [x2, y2], RED).build();
                    draw_list.add_line([x1, y1], [x1, y2], RED).build();
                    draw_list.add_line([x2, y1], [x2, y2], RED).build();
                });
            }
        });

        if options.show_gpu_viewer {
            Gui::draw_gpu_frame(ui, options, video, gpu_frame);
        }

        if options.show_metrics {
            ui.show_metrics_window(&mut options.show_metrics);
        }
    }

    fn draw_gpu_frame(ui: &Ui,
                      options: &mut Options,
                      video: &VideoInterface,
                      gpu_frame: &GpuFrame) {
        ui.window(im_str!("GPU Viewer"))
            .size((300.0, 395.0), ImGuiCond::Once)
            .menu_bar(true)
            .build(|| {
                ui.menu_bar(|| {
                    ui.menu(im_str!("Filter")).build(|| {});
                    ui.menu(im_str!("Options")).build(|| {
                        Gui::menu_item(ui, "Overlay position", &mut options.gpu_viewer.overlay_position);
                        Gui::menu_item(ui, "Overlay texture", &mut options.gpu_viewer.overlay_texture);
                        Gui::menu_item(ui, "Overlay CLUT", &mut options.gpu_viewer.overlay_clut);
                    });
                });

            for i in 0..gpu_frame.commands.len() {
                let command = &gpu_frame.commands[i];
                let command_name = GpuCommand::name(command);
                let title = ImString::new(format!("{}. {}", i, command_name));

                Gui::draw_gpu_command(ui, video, title, command);
            }
        });
    }

    fn draw_gpu_command(ui: &Ui,
                        video: &VideoInterface,
                        title: ImString,
                        command: &GpuCommand) {
        if ui.collapsing_header(&title).build() {
            match command {
                GpuCommand::Polygon(p) => {
                    if !p.shaded {
                        let mut colour = p.vertices[0].colour();
                        let flags = ImGuiColorEditFlags::NoLabel
                                    | ImGuiColorEditFlags::NoPicker
                                    | ImGuiColorEditFlags::NoOptions
                                    | ImGuiColorEditFlags::NoInputs;

                        ui.text("Colour:");

                        ui.same_line(0.0);

                        ui.color_edit(im_str!(""), &mut colour)
                            .flags(flags)
                            .build();
                    }

                    let vertices = if p.quad { 4 } else { 3 };

                    for i in 0..vertices {
                        ui.text(format!("Vertex {}", i + 1));

                        if p.shaded {
                            ui.same_line(0.0);

                            let mut colour = p.vertices[i].colour();
                            let flags = ImGuiColorEditFlags::NoLabel
                                        | ImGuiColorEditFlags::NoPicker
                                        | ImGuiColorEditFlags::NoOptions
                                        | ImGuiColorEditFlags::NoInputs;

                            ui.color_edit(im_str!(""), &mut colour)
                                .flags(flags)
                                .build();
                        }

                        let (x, y) = p.vertices[i].position;
                        ui.text(format!("Position: ({}, {})", x, y));

                        if p.textured {
                            let (u, v) = p.vertices[i].texcoord;
                            ui.text(format!("Texcoord: ({}, {})", u, v));
                        }

                        if i < vertices - 1 {
                            ui.new_line();
                        }
                    }

                    let window_size = video.get_size();

                    let p1 = video.to_screen(p.vertices[0].position());
                    let p2 = video.to_screen(p.vertices[1].position());
                    let p3 = video.to_screen(p.vertices[2].position());
                    let p4 = video.to_screen(p.vertices[3].position());

                    let t1 = video.to_screen(p.vertices[0].texcoord(p.texpage));
                    let t2 = video.to_screen(p.vertices[1].texcoord(p.texpage));
                    let t3 = video.to_screen(p.vertices[2].texcoord(p.texpage));
                    let t4 = video.to_screen(p.vertices[3].texcoord(p.texpage));

                    let draw_list = ui.get_window_draw_list();
                    draw_list.with_clip_rect((0.0, 0.0), window_size, || {
                        draw_list.add_triangle(p1, p2, p3, GREEN_OVERLAY)
                            .filled(true)
                            .build();

                        if p.textured {
                            draw_list.add_triangle(t1, t2, t3, RED_OVERLAY)
                                .filled(true)
                                .build();
                        }

                        if p.quad {
                            draw_list.add_triangle(p2, p3, p4, GREEN_OVERLAY)
                                .filled(true)
                                .build();

                            if p.textured {
                                draw_list.add_triangle(t2, t3, t4, RED_OVERLAY)
                                    .filled(true)
                                    .build();
                            }
                        }
                    });
                },
            };
        }
    }

    fn draw_file_menu(ui: &Ui, system: &System, video: &VideoInterface) {
        if Gui::menu_item(ui, "Dump framebuffer", &mut false) {
            system.dump_vram();
            video.dump_framebuffer();
        }
    }

    fn draw_emu_menu(ui: &Ui, options: &mut Options, system: &mut System) {
        if Gui::menu_item_shortcut(ui, "Reset", "F2", &mut false) {
            system.reset();
        }

        Gui::menu_item_shortcut(ui, "Step", "F3", &mut options.step);
        Gui::menu_item_shortcut(ui, "Pause", "P", &mut options.pause);
        Gui::menu_item_shortcut(ui, "Frame limit", "TAB", &mut options.frame_limit);
    }

    fn draw_debug_menu(ui: &Ui, options: &mut Options) {
        Gui::menu_item(ui, "Draw full VRAM", &mut options.draw_full_vram);
        Gui::menu_item(ui, "GPU Viewer", &mut options.show_gpu_viewer);
    }

    fn draw_view_menu(ui: &Ui, options: &mut Options) {
        Gui::menu_item_enabled(ui, "Draw display area",
                               options.draw_full_vram,
                               &mut options.draw_display_area);

        ui.menu(im_str!("Window scaling")).build(|| {
            let mut selection = options.scaling as i32;
            let items = [
                im_str!("None"),
                im_str!("Aspect"),
                im_str!("Fullscreen")
            ];

            if ui.combo(im_str!(""), &mut selection, &items, 3) {
                options.scaling = Scaling::from(selection);
            }
        });

        Gui::menu_item(ui, "Hardware Rendering", &mut options.hardware);

        ui.menu(im_str!("Hardware Scaling")).build(|| {
            let mut selection = options.hardware_scale as i32;
            let items = [
                im_str!("1x"),
                im_str!("2x"),
                im_str!("4x"),
                im_str!("8x"),
                im_str!("16x"),
            ];

            if ui.combo(im_str!(""), &mut selection, &items, 5) {
                options.hardware_scale = selection as usize;
            }
        });

        Gui::menu_item(ui, "Show metrics", &mut options.show_metrics);
    }

    fn menu_item(ui: &Ui, label: &str, selected: &mut bool) -> bool {
        let l = ImString::new(label);
        ui.menu_item(&l).selected(selected).build()
    }

    fn menu_item_shortcut(ui: &Ui,
                          label: &str,
                          shortcut: &str,
                          selected: &mut bool) -> bool {
        let l = ImString::new(label);
        let s = ImString::new(shortcut);
        ui.menu_item(&l).shortcut(&s).selected(selected).build()
    }

    fn menu_item_enabled(ui: &Ui,
                         label: &str,
                         enabled: bool,
                         selected: &mut bool) -> bool {
        let l = ImString::new(label);
        ui.menu_item(&l).enabled(enabled).selected(selected).build()
    }
}