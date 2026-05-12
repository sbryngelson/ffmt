module m_test

contains

    subroutine s_test()

        ! Old ffmt broke before % - these inputs have that bad wrapping
        @:PROHIBIT(n == 0 .or. p > 0 .or. patch_ib(patch_id)%length_x <= 0._wp .or. patch_ib(patch_id)%length_y <= 0._wp &
                   & .or. f_is_default(patch_ib(patch_id)%x_centroid) .or. f_is_default(patch_ib(patch_id)%y_centroid), &
                   & 'in ellipse IB patch ' // trim(iStr))

        if (((x_cc(i) - x_centroid)**2 + (y_cc(j) - y_centroid)**2 <= radius**2 &
            & .and. patch_icpp(patch_id)%alter_patch(patch_id_fp(i, j, 0))) .or. patch_id_fp(i, j, 0) == smooth_patch_id) then
            call s_assign()
        end if

        q_prim_vf(eqn_idx%E)%sf(j, k, l) = 1._wp/q_prim_vf(eqn_idx%gamma)%sf(j, k, &
                  & l)*(eta*patch_icpp(patch_id)%gamma*patch_icpp(patch_id)%pres + (1._wp - eta) &
                  & *patch_icpp(smooth_patch_id)%gamma*patch_icpp(smooth_patch_id)%pres)

        if (i > 1 .and. (patch_icpp(i)%geometry == 2 .or. patch_icpp(i)%geometry == 3 .or. patch_icpp(i)%geometry == 4 &
            & .or. patch_icpp(i)%geometry == 5 .or. patch_icpp(i)%geometry == 8 .or. patch_icpp(i)%geometry == 9 &
            & .or. patch_icpp(i)%geometry == 10 .or. patch_icpp(i)%geometry == 11)) then
            call s_check()
        end if

    end subroutine s_test

end module m_test
