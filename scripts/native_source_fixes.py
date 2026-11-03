"""Warning fixes for the pinned libde265 1.0.15 sources.

Debug, assertion-only, and experimental declarations remain available, marked
with C++17 maybe_unused. Compiler diagnostics stay enabled. Other fixes use
signed SIMD masks, bounded formatting, consistent class tags, and initialized
motion-vector costs, and remove an always-true condition.
"""

# Exact declarations keep these edits scoped to the pinned upstream release.
UNUSED_DECLARATIONS = {
    'libde265/bitstream.cc': [
        '  int stop_bit = get_bits(br,1);',
    ],
    'libde265/cabac.cc': [
        '      int num_bitsTab = renorm_table[ LPS >> 3 ];',
        'const uint32_t entropy_table_theory[128] =',
        'static const uint32_t entropy_table_orig[128] = {',
    ],
    'libde265/de265.cc': [
        'static void dumpdata(const void* data, int len)',
    ],
    'libde265/deblock.cc': [
        '  decoder_context* ctx = img->decctx;',
        '  int TUShift = img->get_sps().Log2MinTrafoSize;',
        '  int TUStride= img->get_sps().PicWidthInTbsY;',
        '  int n=0;',
    ],
    'libde265/decctx.cc': [
        '  bool waitForCompletion = false;',
        '  de265_error err = DE265_OK;',
    ],
    'libde265/en265.cc': [
        '  encoder_context* ectx = (encoder_context*)e;',
    ],
    'libde265/encoder/algo/cb-interpartmode.cc': [
        '  int nPB;',
    ],
    'libde265/encoder/algo/cb-intra-inter.cc': [
        '  bool debug_halt = try_inter;',
    ],
    'libde265/encoder/algo/cb-mergeindex.cc': [
        '    int tbSize = 1<<cb->log2Size;',
        '  const de265_image* refimg = ectx->get_image(ectx->imgdata->frame_number -1);',
        '  int IntraSplitFlag = 0;',
        '  int MaxTrafoDepth = ectx->get_sps().max_transform_hierarchy_depth_inter;',
    ],
    'libde265/encoder/algo/pb-mv.cc': [
        '    const de265_image* input = ectx->imgdata->input;',
        '    int tbSize = 1<<cb->log2Size;',
        '  enum MVSearchAlgo searchAlgo = mParams.mvSearchAlgo();',
        '  int IntraSplitFlag = 0;',
        '  int MaxTrafoDepth = ectx->get_sps().max_transform_hierarchy_depth_inter;',
    ],
    'libde265/encoder/algo/tb-intrapredmode.cc': [
        '    const seq_parameter_set* sps = &ectx->get_sps();',
        '    float minCandCost;',
        '    float minCost = std::numeric_limits<float>::max();',
        '    int   minCostIdx=0;',
        '  float distortion;',
        '  int xBase = cb->x;',
        '  int yBase = cb->y;',
    ],
    'libde265/encoder/algo/tb-split.cc': [
        '  const de265_image* img = ectx->img;',
        '  enum IntraPredMode mode;',
        '  int tbSize = 1<<tb->log2Size;',
    ],
    'libde265/encoder/algo/tb-transform.cc': [
        '  de265_image* img = ectx->img;',
        '  int stride = ectx->img->get_image_stride(0);',
        '  int xC = x0;',
        '  int yC = y0;',
        '  int16_t blk[32*32]; // residual',
        '  uint8_t* cb_plane = ectx->img->get_image_plane(1);',
        '  uint8_t* cr_plane = ectx->img->get_image_plane(2);',
        '  uint8_t* luma_plane = ectx->img->get_image_plane(0);',
    ],
    'libde265/encoder/encoder-context.cc': [
        '  double psnr = encode_image(this,imgdata->input, algo);',
    ],
    'libde265/encoder/encoder-core.cc': [
        '  int stride=input->get_image_stride(0);',
        '  uint8_t* cb_plane   = ectx->img->get_image_plane(1);',
        '  uint8_t* cr_plane   = ectx->img->get_image_plane(2);',
        '  uint8_t* luma_plane = ectx->img->get_image_plane(0);',
    ],
    'libde265/encoder/encoder-syntax.cc': [
        '        int PUidx = (x0>>sps.Log2MinPUSize) + (y0>>sps.Log2MinPUSize)*sps.PicWidthInMinPUs;',
        '        int PUidx;',
        '      bool split_transform_flag = (log2TrafoSize > sps.Log2MaxTrafoSize ||',
        '      int sumAbsLevel=0;',
    ],
    'libde265/encoder/encoder-types.cc': [
        '        enum IntraPredMode intraPredMode  = intra_mode;',
        '        int dst_stride  = img->get_image_stride(cIdx);',
        '        int size = 1<<log2TbSize;',
        '        uint8_t* dst_ptr  = img->get_image_plane_at_pos(cIdx, xC,  yC  );',
    ],
    'libde265/encoder/sop.cc': [
        '  int poc = get_pic_order_count();',
    ],
    'libde265/fallback-dct.cc': [
        '  int bdShift2 = 20-bit_depth;',
        'static void printMatrix(const char* name, const int16_t* v, int n)',
    ],
    'libde265/image.cc': [
        'static const int alignment = 16;',
    ],
    'libde265/motion.cc': [
        '      const de265_image* refPicX = ctx->get_image(shdr->RefPicList[X         ][refIdxLX]);',
        '  int nOrigPbH = nPbH;',
        '  int nOrigPbW = nPbW;',
    ],
    'libde265/sao.cc': [
        '  int n=0;',
    ],
    'libde265/sei.cc': [
        'static inline uint16_t crc_process_byte(uint16_t crc, uint8_t byte)',
    ],
    'libde265/slice.cc': [
        '    int ctby = tctx->CtbY;',
        '  const int yOffset422 = 1<<log2TrafoSizeC;',
        '  const seq_parameter_set& sps = img->get_sps();',
        '  decoder_context* ctx = tctx->decctx;',
        '  int sumWeightFlags = 0;',
        '  int xL = x0 - xCUBase;',
        '  int yL = y0 - yCUBase;',
        'static void dump_cbsize(de265_image* img)',
    ],
    'libde265/transform.cc': [
        '    int bdShift2 = (cIdx==0) ? 20-sps.BitDepth_Y : 20-sps.BitDepth_C;',
        '  int blkSize = (1<<log2TrSize);',
        '  int uiAcSum = 0;',
    ],
    'libde265/util.cc': [
        'static long logcnt[10];',
    ],
    'libde265/x86/sse-motion.cc': [
        'static const int epel_extra_after = 2;',
        'static const uint8_t qpel_extra_after[4] = { 0, 3, 4, 4 };',
    ],
}

SOURCE_FIXES = {
    'CMakeLists.txt': {
        'cmake_minimum_required (VERSION 3.3.2)': 'cmake_minimum_required (VERSION 3.3.2...3.10)',
        'set(CMAKE_CXX_STANDARD 11)': 'set(CMAKE_CXX_STANDARD 17)',
    },
    'libde265/deblock.cc': {
        'if (vertical || !vertical) {': '{ // Both vertical and horizontal edges.',
        '        else {\n          // TODO\n          assert(0);\n        }':
            '        // The alternative branch was unreachable.',
        'sprintf(buf,': 'snprintf(buf, sizeof(buf),',
    },
    'libde265/decctx.cc': {
        'struct thread_context tctx;': 'class thread_context tctx;',
    },
    'libde265/encoder/algo/pb-mv.cc': {
        'else { b=abs_value(b+2); }': 'else { b=abs_value(diff)+2; }',
    },
    'libde265/sao.cc': {
        'sprintf(buf,': 'snprintf(buf, sizeof(buf),',
    },
    'libde265/slice.cc': {
        'sprintf(buf,': 'snprintf(buf, sizeof(buf),',
    },
    'libde265/x86/sse-motion.cc': {
        '_mm_set_epi16(0, 65535, 0, 65535, 0, 65535, 0, 65535)': '_mm_set_epi16(0, -1, 0, -1, 0, -1, 0, -1)',
    },
}


def apply_libde265_fixes(source):
    edits = {name: replacements.copy() for name, replacements in SOURCE_FIXES.items()}
    for name, declarations in UNUSED_DECLARATIONS.items():
        for declaration in declarations:
            indent = declaration[:len(declaration) - len(declaration.lstrip())]
            edits.setdefault(name, {})[declaration] = indent + "[[maybe_unused]] " + declaration.lstrip()
    for relative, replacements in edits.items():
        path = source / relative
        contents = path.read_text()
        for original, updated in replacements.items():
            if updated in contents:
                continue
            if original not in contents:
                raise RuntimeError(f"Unexpected libde265 source in {relative}: {original}")
            contents = contents.replace(original, updated)
        if path.read_text() != contents:
            path.write_text(contents)
