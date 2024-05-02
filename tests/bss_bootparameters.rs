// tests for BSS
#[cfg(test)]
mod tests {
    use mesa::bss::bootparameters::BootParameters;

    #[test]
    fn test_update_boot_image_ncn() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "ifname=mgmt0:14:02:ec:e3:cb:80 ifname=sun0:14:02:ec:e3:cb:81 ifname=mgmt1:b4:7a:f1:fe:63:16 ifname=sun1:b4:7a:f1:fe:63:17 biosdevname=1 pcie_ports=native transparent_hugepage=never console=tty0 console=ttyS0,115200 iommu=pt metal.server=s3://boot-images/28fa52c1-1e1b-4337-9a60-6466c81e7300/rootfs metal.no-wipe=1 ds=nocloud-net;s=http://10.92.100.81:8888/ rootfallback=LABEL=BOOTRAID initrd=initrd.img.xz root=live:LABEL=SQFSRAID rd.live.ram=0 rd.writable.fsimg=0 rd.skipfsck rd.live.overlay=LABEL=ROOTRAID rd.live.overlay.overlayfs=1 rd.luks rd.luks.crypttab=0 rd.lvm.conf=0 rd.lvm=1 rd.auto=1 rd.md=1 rd.dm=0 rd.neednet=0 rd.md.waitclean=1 rd.multipath=0 rd.md.conf=1 rd.bootif=0 hostname=ncn-s005 rd.net.timeout.carrier=120 rd.net.timeout.ifup=120 rd.net.timeout.iflink=120 rd.net.timeout.ipv6auto=0 rd.net.timeout.ipv6dad=0 append nosplash quiet crashkernel=360M log_buf_len=1 rd.retry=10 rd.shell ip=mgmt0:dhcp rd.peerdns=0 rd.net.dhcp.retry=5 psi=1 split_lock_detect=off rd.live.squashimg=rootfs rd.live.overlay.thin=0 rd.live.dir=1.5.0".to_string(),
            kernel: "s3://boot-images/28fa52c1-1e1b-4337-9a60-6466c81e7300/kernel".to_string(),
            initrd: "s3://boot-images/28fa52c1-1e1b-4337-9a60-6466c81e7300/initrd".to_string(),
            cloud_init: None,
        };

        let new_image_id = "my_new_image";

        boot_parameters.set_boot_image(new_image_id);

        let kernel_param_iter = boot_parameters.params.split_whitespace();

        let mut pass = true;

        for kernel_param in kernel_param_iter {
            if kernel_param.contains("metal.server=s3://boot-images/") {
                // println!("DEBUG - kernel param: {}", kernel_param);
                pass = pass && kernel_param.contains(new_image_id);
            }

            if kernel_param.contains("root=craycps-s3:s3://boot-images/") {
                // println!("DEBUG - kernel param: {}", kernel_param);
                pass = pass && kernel_param.contains(new_image_id);
            }

            if kernel_param.contains("nmd_data=url=s3://boot-images/") {
                // println!("DEBUG - kernel param: {}", kernel_param);
                pass = pass && kernel_param.contains(new_image_id);
            }
        }

        pass = pass && boot_parameters.kernel.contains(new_image_id);
        pass = pass && boot_parameters.initrd.contains(new_image_id);

        assert!(pass)
    }

    #[test]
    fn test_update_boot_image_cn() {
        let mut boot_parameters = BootParameters {
            hosts: vec![],
            macs: None,
            nids: None,
            params: "console=ttyS0,115200 bad_page=panic crashkernel=360M hugepagelist=2m-2g intel_iommu=off intel_pstate=disable iommu.passthrough=on numa_interleave_omit=headless oops=panic pageblock_order=14 rd.neednet=1 rd.retry=10 rd.shell dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN} root=craycps-s3:s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs:350a27edb711cbcd8cff27470711f841-317:dvs:api-gw-service-nmn.local:300:nmn0 nmd_data=url=s3://boot-images/6c644208-104a-473d-802c-410219026335/rootfs,etag=350a27edb711cbcd8cff27470711f841-317 bos_session_id=50c401a9-3324-4844-bf82-872adb0ebe6f".to_string(),
            kernel: "s3://boot-images/6c644208-104a-473d-802c-410219026335/kernel".to_string(),
            initrd: "s3://boot-images/6c644208-104a-473d-802c-410219026335/initrd".to_string(),
            cloud_init: None,
        };

        let new_image_id = "my_new_image";

        boot_parameters.set_boot_image(new_image_id);

        let kernel_param_iter = boot_parameters.params.split_whitespace();

        let mut pass = true;

        for kernel_param in kernel_param_iter {
            if kernel_param.contains("metal.server=s3://boot-images/") {
                pass = pass && kernel_param.contains(new_image_id);
            }

            if kernel_param.contains("root=craycps-s3:s3://boot-images/") {
                pass = pass && kernel_param.contains(new_image_id);
            }

            if kernel_param.contains("nmd_data=url=s3://boot-images/") {
                pass = pass && kernel_param.contains(new_image_id);
            }
        }

        pass = pass && boot_parameters.kernel.contains(new_image_id);
        pass = pass && boot_parameters.initrd.contains(new_image_id);

        assert!(pass)
    }
}
