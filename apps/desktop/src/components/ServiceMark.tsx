import { providerLogos } from '../../../../packages/provider-registry/logos';

export function ServiceMark({
  name,
  presetId = 'custom',
  large = false,
}: {
  name: string;
  presetId?: string;
  large?: boolean;
}) {
  return (
    <span
      className={`service-mark ${large ? 'large' : ''}`}
      data-service={presetId}
      aria-hidden="true"
    >
      {providerLogos[presetId] ? (
        <img src={providerLogos[presetId]} alt="" />
      ) : presetId === 'custom' ? (
        '>_'
      ) : (
        name.slice(0, 1)
      )}
    </span>
  );
}
